use super::column::MetaCol;
use super::cursor::CursorQuery;
use super::template::TemplateFile;
use crate::database::datatype::{ColDataType, ColDef, DataTypeQuery};
use crate::database::dialect::SqlDialect;
use crate::database::path::clean_and_ensure_path;
use crate::impl_odbc_provider;
use crate::trnsys::error::TrnSysError;
use indexmap::IndexSet;
use odbc_api::parameter::InputParameter;
use odbc_api::{BindParamDesc, IntoParameter};
use odbc_api::sys::{Date, Time, Timestamp};
use odbc_api::{Connection, ConnectionOptions, Cursor, DataType, Environment, ResultSetMetadata};
use std::fs;
use std::sync::{Mutex, MutexGuard};
use strum::IntoEnumIterator;
use tracing::{debug, info};

pub const VARIANTS_TABLE: &str = "variants";
pub const VARIANT_ID_COL: &str = "variant_id";
pub const VARIANT_NAME_COL: &str = "variant_name";
pub const COMPLETE_COL: &str = "complete";
pub const CREATED_AT_COL: &str = "created_at";
pub const UPDATED_AT_COL: &str = "updated_at";

/// Map an ODBC `DataType` (from catalog/result-set metadata) to our generic
/// `ColDataType` so we can regenerate a CREATE TABLE statement for a table
/// whose schema we discovered at runtime.
fn map_odbc_data_type(dt: &DataType) -> ColDataType {
    match dt {
        DataType::TinyInt | DataType::SmallInt | DataType::Integer | DataType::BigInt => {
            ColDataType::Number { decimal: false }
        }
        DataType::Float { .. }
        | DataType::Real
        | DataType::Double
        | DataType::Decimal { .. }
        | DataType::Numeric { .. } => ColDataType::Number { decimal: true },
        DataType::Bit => ColDataType::Boolean,
        DataType::Date | DataType::Timestamp { .. } | DataType::Time { .. } => ColDataType::DateTime,
        _ => ColDataType::Text,
    }
}

/// Scan the variants table and return the `variant_name` for the given id.
/// Standalone helper so callers can reuse an already-held connection guard.
fn lookup_variant_name_with_conn(
    conn: &MutexGuard<Connection>,
    variants_ref: &str,
    variant_id: i32,
) -> Result<Option<String>, TrnSysError> {
    let select = format!("SELECT * FROM {}", variants_ref);
    let Some(mut cursor) = conn.execute(&select, (), None)? else {
        return Ok(None);
    };
    let num_cols = cursor.num_result_cols()? as u16;
    let mut id_idx: Option<u16> = None;
    let mut name_idx: Option<u16> = None;
    for i in 1..=num_cols {
        let col_name = cursor.col_name(i)?;
        if col_name.eq_ignore_ascii_case(VARIANT_ID_COL) {
            id_idx = Some(i);
        } else if col_name.eq_ignore_ascii_case(VARIANT_NAME_COL) {
            name_idx = Some(i);
        }
    }
    let (Some(id_idx), Some(name_idx)) = (id_idx, name_idx) else {
        return Ok(None);
    };
    while let Some(mut row) = cursor.next_row()? {
        let mut id: i32 = 0;
        if row.get_data(id_idx, &mut id).is_err() {
            continue;
        }
        if id == variant_id {
            let mut buf = Vec::new();
            if row.get_text(name_idx, &mut buf)? {
                if let Ok(name) = std::str::from_utf8(&buf) {
                    return Ok(Some(name.to_string()));
                }
            }
        }
    }
    Ok(None)
}

pub trait OdbcProvider<'c>: Send + Sync + SqlDialect {
    fn set_connection(&mut self, connection: Connection<'c>) -> Result<(), TrnSysError>;
    fn setup_by_conn_str(
        &mut self,
        environment: &'c Environment,
        conn_str: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        debug!("Connection String: {}", conn_str);
        let connection = environment
            .connect_with_connection_string(conn_str, conn_options.unwrap_or_default())?;
        self.set_connection(connection)?;
        Ok(())
    }
    #[allow(dead_code)]
    fn setup_by_dsn(
        &mut self,
        environment: &'c Environment,
        dsn: &str,
        user: &str,
        password: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let connection =
            environment.connect(dsn, user, password, conn_options.unwrap_or_default())?;
        self.set_connection(connection)?;
        Ok(())
    }

    fn get_connection(&self) -> Result<MutexGuard<Connection<'c>>, TrnSysError>;

    fn ensure_table(
        &self,
        table_name: &str,
        cols: Vec<ColDef>,
        creation_extra_cols: Option<Vec<String>>,
    ) -> Result<(), TrnSysError> {
        let connection = self.get_connection()?;

        let mut col_type_set: IndexSet<ColDef> = cols.into_iter().collect();

        // Add predefined columns in the front
        for (i, meta_col) in MetaCol::iter().enumerate() {
            col_type_set.insert_before(i, meta_col.col_def());
        }
        debug!("table_name: {}", table_name);
        // Check if table exists
        let mut table_exists = false;
        for row in connection.tables("", "", table_name, "TABLE")? {
            let row = row?;
            if let Ok(Some(name)) = row.table.as_str() {
                debug!("Found Table: {}", name);
                if name == table_name {
                    table_exists = true;
                    break;
                }
            }
        }

        debug!("Table exists: {}", table_exists);
        if table_exists {
            // Remove existing columns from the set so only missing ones remain
            for row in connection.columns("", "", table_name, "")? {
                let row = row?;
                if let Ok(Some(column_name)) = row.column_name.as_str() {
                    col_type_set.shift_remove(&ColDef::new(
                        column_name,
                        ColDataType::Text,
                        false,
                        false,
                    ));
                }
            }

            // add missing columns
            for col_def in col_type_set {
                let alter_query = format!(
                    "ALTER TABLE {} ADD COLUMN {}",
                    table_name,
                    self.get_col_def_str(&col_def)
                );
                connection.execute(&alter_query, (), None)?;
            }
        } else {
            // add a new table

            let primary_key_str = self.get_primary_key_str(col_type_set.iter().collect());

            let cols_def = col_type_set
                .iter()
                .map(|col| self.get_col_def_str(col))
                .chain(creation_extra_cols.unwrap_or_default())
                .chain(vec![primary_key_str])
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", \n");

            let create_table_query = format!(
                r"
            CREATE TABLE {}
            (
            {}
            );",
                table_name, cols_def
            );
            debug!("Create Table Query: {}", create_table_query);
            connection.execute(&create_table_query, (), None)?;

            // Add a non-unique index on variant_id for fast DELETE/SELECT by variant.
            // Best-effort: some backends (e.g. Excel) don't support CREATE INDEX.
            let index_name = format!("idx_{}_variant_id", table_name);
            let index_query = format!(
                "CREATE INDEX {} ON {} ({})",
                self.format_identifier(&index_name),
                self.format_identifier(table_name),
                self.format_identifier(MetaCol::VariantId.as_str()),
            );
            debug!("Create Index Query: {}", index_query);
            if let Err(e) = connection.execute(&index_query, (), None) {
                debug!("CREATE INDEX skipped ({}): {}", index_name, e);
            }
        }
        Ok(())
    }

    /// Create the variants lookup table if it doesn't exist.
    /// Schema: variant_id AUTOINCREMENT PK, variant_name TEXT, created_at DATETIME, updated_at DATETIME.
    fn ensure_variants_table(&self) -> Result<(), TrnSysError> {
        let connection = self.get_connection()?;

        let mut exists = false;
        for row in connection.tables("", "", VARIANTS_TABLE, "TABLE")? {
            let row = row?;
            if let Ok(Some(name)) = row.table.as_str() {
                if name == VARIANTS_TABLE {
                    exists = true;
                    break;
                }
            }
        }
        if exists {
            return Ok(());
        }

        let other_cols = vec![
            ColDef::new(VARIANT_NAME_COL, ColDataType::Text, true, false),
            ColDef::new(COMPLETE_COL, ColDataType::Boolean, true, false),
            ColDef::new(CREATED_AT_COL, ColDataType::DateTime, true, false),
            ColDef::new(UPDATED_AT_COL, ColDataType::DateTime, true, false),
        ];
        let cols_def = std::iter::once(self.autoincrement_pk_def(VARIANT_ID_COL))
            .chain(other_cols.iter().map(|c| self.get_col_def_str(c)))
            .collect::<Vec<_>>()
            .join(", \n");
        let create = format!(
            "CREATE TABLE {} ({})",
            self.format_identifier(VARIANTS_TABLE),
            cols_def
        );
        debug!("Create Variants Table: {}", create);
        connection.execute(&create, (), None)?;
        Ok(())
    }

    /// Look up a variant by name; insert if missing. Returns the variant_id.
    /// Updates `updated_at` on each call so the lookup table tracks last use.
    fn ensure_variant(&self, variant_name: &str) -> Result<i32, TrnSysError> {
        if !self.supports_autoincrement() {
            return self.ensure_variant_direct(variant_name);
        }

        let conn = self.get_connection()?;

        let select = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            self.format_identifier(VARIANT_ID_COL),
            self.format_identifier(VARIANTS_TABLE),
            self.format_identifier(VARIANT_NAME_COL),
        );

        let lookup = |conn: &MutexGuard<Connection>| -> Result<Option<i32>, TrnSysError> {
            let mut stmt = conn.prepare(&select)?;
            let params: Vec<Box<dyn InputParameter>> =
                vec![Box::new(variant_name.to_string().into_parameter())];
            let cursor = stmt.execute(params.as_slice())?;
            match cursor {
                Some(mut cursor) => match cursor.next_row()? {
                    Some(mut row) => {
                        let mut id: i32 = 0;
                        if row.get_data(1, &mut id).is_ok() {
                            Ok(Some(id))
                        } else {
                            Ok(None)
                        }
                    }
                    None => Ok(None),
                },
                None => Ok(None),
            }
        };

        if let Some(id) = lookup(&conn)? {
            let update = format!(
                "UPDATE {} SET {} = 0, {} = {} WHERE {} = ?",
                self.format_identifier(VARIANTS_TABLE),
                self.format_identifier(COMPLETE_COL),
                self.format_identifier(UPDATED_AT_COL),
                self.current_timestamp_expr(),
                self.format_identifier(VARIANT_ID_COL),
            );
            let mut stmt = conn.prepare(&update)?;
            let params: Vec<Box<dyn InputParameter>> = vec![Box::new(id.into_parameter())];
            stmt.execute(params.as_slice())?;
            debug!("Variant '{}' already exists with id {}", variant_name, id);
            return Ok(id);
        }

        // Insert — variant_id is auto-generated by the DB
        let ts = self.current_timestamp_expr();
        let insert = format!(
            "INSERT INTO {} ({}, {}, {}, {}) VALUES (?, 0, {ts}, {ts})",
            self.format_identifier(VARIANTS_TABLE),
            self.format_identifier(VARIANT_NAME_COL),
            self.format_identifier(COMPLETE_COL),
            self.format_identifier(CREATED_AT_COL),
            self.format_identifier(UPDATED_AT_COL),
        );
        let mut stmt = conn.prepare(&insert)?;
        let params: Vec<Box<dyn InputParameter>> =
            vec![Box::new(variant_name.to_string().into_parameter())];
        stmt.execute(params.as_slice())?;

        let id = lookup(&conn)?.ok_or_else(|| {
            TrnSysError::GeneralError(format!(
                "Failed to retrieve auto-generated variant_id for '{}'",
                variant_name
            ))
        })?;
        info!("Inserted new variant '{}' with id {}", variant_name, id);
        Ok(id)
    }

    /// Variant management using inline literals (SQLExecDirect).
    /// Used by dialects whose ODBC drivers mishandle prepared statements against
    /// freshly-created tables — notably Excel, which treats unknown column
    /// identifiers as implicit parameter markers when no data rows exist yet.
    fn ensure_variant_direct(&self, variant_name: &str) -> Result<i32, TrnSysError> {
        let conn = self.get_connection()?;
        let name_lit = self.format_text_literal(variant_name);
        let ts = self.current_timestamp_expr();

        // Excel references sheets as `[name$]`; named ranges created via
        // CREATE TABLE are not always discoverable until reconnect, so use the
        // sheet form here. `format_data_table` is a no-op for other dialects.
        let variants_ref = self.format_data_table(VARIANTS_TABLE);

        // Lookup by name. SELECT * avoids Excel's quirk of treating unknown
        // column identifiers as parameter markers when the sheet has 0 rows.
        let select = format!("SELECT * FROM {}", variants_ref);
        let mut existing_id: Option<i32> = None;
        let mut max_id: i32 = 0;
        if let Some(mut cursor) = conn.execute(&select, (), None)? {
            // Resolve column indices from the result-set metadata by name.
            let num_cols = cursor.num_result_cols()? as u16;
            let mut id_idx: Option<u16> = None;
            let mut name_idx: Option<u16> = None;
            for i in 1..=num_cols {
                let col_name = cursor.col_name(i)?;
                if col_name.eq_ignore_ascii_case(VARIANT_ID_COL) {
                    id_idx = Some(i);
                } else if col_name.eq_ignore_ascii_case(VARIANT_NAME_COL) {
                    name_idx = Some(i);
                }
            }
            let id_idx = id_idx.ok_or_else(|| {
                TrnSysError::GeneralError(format!(
                    "variants table missing column '{}'",
                    VARIANT_ID_COL
                ))
            })?;
            let name_idx = name_idx.ok_or_else(|| {
                TrnSysError::GeneralError(format!(
                    "variants table missing column '{}'",
                    VARIANT_NAME_COL
                ))
            })?;

            while let Some(mut row) = cursor.next_row()? {
                let mut id: i32 = 0;
                if row.get_data(id_idx, &mut id).is_err() {
                    continue;
                }
                if id > max_id {
                    max_id = id;
                }
                let mut buf = Vec::new();
                if row.get_text(name_idx, &mut buf)? {
                    if let Ok(name) = std::str::from_utf8(&buf) {
                        if name == variant_name {
                            existing_id = Some(id);
                        }
                    }
                }
            }
        }

        if let Some(id) = existing_id {
            let update = format!(
                "UPDATE {} SET {} = 0, {} = {} WHERE {} = {}",
                variants_ref,
                self.format_identifier(COMPLETE_COL),
                self.format_identifier(UPDATED_AT_COL),
                ts,
                self.format_identifier(VARIANT_NAME_COL),
                name_lit,
            );
            conn.execute(&update, (), None)?;
            debug!("Variant '{}' already exists with id {}", variant_name, id);
            return Ok(id);
        }

        let next_id = max_id + 1;
        let insert = format!(
            "INSERT INTO {} ({}, {}, {}, {}, {}) VALUES ({}, {}, 0, {ts}, {ts})",
            variants_ref,
            self.format_identifier(VARIANT_ID_COL),
            self.format_identifier(VARIANT_NAME_COL),
            self.format_identifier(COMPLETE_COL),
            self.format_identifier(CREATED_AT_COL),
            self.format_identifier(UPDATED_AT_COL),
            next_id,
            name_lit,
        );
        conn.execute(&insert, (), None)?;
        info!("Inserted new variant '{}' with id {}", variant_name, next_id);
        Ok(next_id)
    }

    /// Mark a variant as complete (sets complete = 1, updates updated_at).
    fn mark_variant_complete(&self, variant_id: i32) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;
        let variants_ref = self.format_data_table(VARIANTS_TABLE);
        let ts = self.current_timestamp_expr();

        // Excel stores cells with loose typing; `WHERE variant_id = <int>` can
        // fail with "Data type mismatch" even when the column was declared as
        // NUMBER. Match by variant_name instead for dialects without
        // autoincrement (the variant_name is guaranteed unique in our schema).
        let where_clause = if self.supports_autoincrement() {
            format!("{} = {}", self.format_identifier(VARIANT_ID_COL), variant_id)
        } else {
            let name = lookup_variant_name_with_conn(&conn, &variants_ref, variant_id)?
                .ok_or_else(|| {
                    TrnSysError::GeneralError(format!(
                        "Cannot mark complete: variant_id {} not found",
                        variant_id
                    ))
                })?;
            format!(
                "{} = {}",
                self.format_identifier(VARIANT_NAME_COL),
                self.format_text_literal(&name)
            )
        };

        let update = format!(
            "UPDATE {} SET {} = 1, {} = {} WHERE {}",
            variants_ref,
            self.format_identifier(COMPLETE_COL),
            self.format_identifier(UPDATED_AT_COL),
            ts,
            where_clause,
        );
        conn.execute(&update, (), None)?;
        info!("Marked variant_id={} as complete", variant_id);
        Ok(())
    }


    /// Delete all rows for the given variant_id from the data table.
    /// On dialects without DELETE support (e.g. Excel), the table is rebuilt:
    /// rows for other variants are read, the table is dropped and recreated,
    /// and the preserved rows are reinserted.
    fn remove_variant_data(&self, table_name: &str, variant_id: i32) -> Result<(), TrnSysError> {
        if !self.supports_delete() {
            return self.rebuild_table_preserving_other_variants(table_name, variant_id);
        }
        let conn = self.get_connection()?;
        let query = format!(
            "DELETE FROM {} WHERE {} = {}",
            self.format_data_table(table_name),
            self.format_identifier(MetaCol::VariantId.as_str()),
            variant_id,
        );
        info!(
            "Remove Variant Data: {} (variant_id={})",
            query, variant_id
        );
        conn.execute(&query, (), None)?;
        info!("Variant data removed.");
        Ok(())
    }

    /// Read all rows for variants != `variant_id`, drop the table, recreate it
    /// with the same schema (discovered from ODBC column metadata), and
    /// reinsert the preserved rows. Used for Excel where ISAM blocks DELETE.
    fn rebuild_table_preserving_other_variants(
        &self,
        table_name: &str,
        variant_id: i32,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;
        let data_table_ref = self.format_data_table(table_name);

        // Read schema (names + types) and preserved rows in a single SELECT,
        // using the cursor's result-set metadata. Avoids a separate catalog
        // lookup and sidesteps Excel's i16 SQL-type encoding.
        let select = format!("SELECT * FROM {}", data_table_ref);
        let mut col_defs: Vec<ColDef> = Vec::new();
        let mut col_order: Vec<String> = Vec::new();
        let mut preserved_rows: Vec<Vec<Option<String>>> = Vec::new();
        if let Some(mut cursor) = conn.execute(&select, (), None)? {
            let num_cols = cursor.num_result_cols()? as u16;
            let mut variant_id_idx: Option<u16> = None;
            for i in 1..=num_cols {
                let name = cursor.col_name(i)?;
                let dt = cursor.col_data_type(i)?;
                if name.eq_ignore_ascii_case(MetaCol::VariantId.as_str()) {
                    variant_id_idx = Some(i);
                }
                col_order.push(name.clone());
                col_defs.push(ColDef::new(&name, map_odbc_data_type(&dt), false, false));
            }
            let variant_id_idx = variant_id_idx.ok_or_else(|| {
                TrnSysError::GeneralError(format!(
                    "Cannot preserve rows: {} column not found in {}",
                    MetaCol::VariantId.as_str(),
                    table_name
                ))
            })?;
            while let Some(mut row) = cursor.next_row()? {
                // Some drivers (Excel) require reading columns in ascending
                // order, so read every cell then filter on variant_id afterward.
                let mut values: Vec<Option<String>> = Vec::with_capacity(num_cols as usize);
                let mut vid: Option<i32> = None;
                for i in 1..=num_cols {
                    let dt = &col_defs[(i - 1) as usize].data_type;
                    let v: Option<String> = match dt {
                        ColDataType::Number { decimal: false } | ColDataType::Boolean => {
                            let mut n: i64 = 0;
                            match row.get_data(i, &mut n) {
                                Ok(_) => Some(n.to_string()),
                                Err(_) => None,
                            }
                        }
                        ColDataType::Number { decimal: true } => {
                            let mut f: f64 = 0.0;
                            match row.get_data(i, &mut f) {
                                Ok(_) => Some(f.to_string()),
                                Err(_) => None,
                            }
                        }
                        _ => {
                            let mut buf = Vec::new();
                            match row.get_text(i, &mut buf) {
                                Ok(true) => Some(String::from_utf8_lossy(&buf).into_owned()),
                                _ => None,
                            }
                        }
                    };
                    if i == variant_id_idx {
                        vid = v.as_deref().and_then(|s| s.parse::<i32>().ok());
                    }
                    values.push(v);
                }
                if vid == Some(variant_id) {
                    continue; // discard rows belonging to the current variant
                }
                preserved_rows.push(values);
            }
        }
        if col_defs.is_empty() {
            debug!("Table {} has no columns or does not exist; skipping rebuild", table_name);
            return Ok(());
        }
        info!(
            "Rebuilding {} — preserving {} rows from other variants (discarding variant_id={})",
            table_name,
            preserved_rows.len(),
            variant_id
        );

        // 3. DROP the table and CREATE it again with the same schema.
        let drop_sql = format!("DROP TABLE {}", self.format_identifier(table_name));
        conn.execute(&drop_sql, (), None)?;
        let create_cols = col_defs
            .iter()
            .map(|c| self.get_col_def_str(c))
            .collect::<Vec<_>>()
            .join(", ");
        let create_sql = format!(
            "CREATE TABLE {} ({})",
            self.format_identifier(table_name),
            create_cols
        );
        conn.execute(&create_sql, (), None)?;

        // 4. Reinsert preserved rows, mapping each string back to a typed SQL literal.
        if !preserved_rows.is_empty() {
            // Map introspected column names (case-sensitive as stored) to their ColDef.
            let col_type_by_name: std::collections::HashMap<String, ColDataType> = col_defs
                .iter()
                .map(|c| (c.name.clone(), c.data_type.clone()))
                .collect();
            let insert_cols = col_order
                .iter()
                .map(|n| self.format_identifier(n))
                .collect::<Vec<_>>()
                .join(", ");
            let insert_prefix = format!(
                "INSERT INTO {} ({}) VALUES ",
                data_table_ref, insert_cols
            );
            for values in &preserved_rows {
                let parts: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(idx, v)| match v {
                        None => "NULL".to_string(),
                        Some(s) => {
                            let col_name = &col_order[idx];
                            let dt = col_type_by_name
                                .get(col_name)
                                .cloned()
                                .unwrap_or(ColDataType::Text);
                            match dt {
                                ColDataType::Text => self.format_text_literal(s),
                                ColDataType::DateTime => format!("#{}#", s),
                                ColDataType::Number { .. } | ColDataType::Boolean => {
                                    if s.is_empty() {
                                        "NULL".to_string()
                                    } else {
                                        s.clone()
                                    }
                                }
                            }
                        }
                    })
                    .collect();
                let sql = format!("{}({})", insert_prefix, parts.join(", "));
                conn.execute(&sql, (), None)?;
            }
        }

        Ok(())
    }

    /// Bulk-insert rows using ODBC columnar array parameter binding.
    /// Sends all rows in a single SQLExecute call instead of one per row.
    ///
    /// Column layout (fixed): variant_id_col (i32), simtime_col (f64), input_col_names... (f64).
    fn columnar_batch_insert(
        &self,
        table: &str,
        variant_id_col: &str,
        simtime_col: &str,
        input_col_names: &[String],
        variant_id: i32,
        sim_times: &[f64],
        input_rows: &[Vec<f64>],
    ) -> Result<(), TrnSysError> {
        let n = sim_times.len();
        if n == 0 {
            return Ok(());
        }
        let conn = self.get_connection()?;

        // Build INSERT query with all column names
        let col_name_field = std::iter::once(variant_id_col)
            .chain(std::iter::once(simtime_col))
            .chain(input_col_names.iter().map(String::as_str))
            .map(|name| self.format_identifier(name))
            .collect::<Vec<_>>()
            .join(", ");
        let num_cols = 2 + input_col_names.len();
        let placeholders = vec!["?"; num_cols].join(", ");
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, col_name_field, placeholders
        );
        debug!("Columnar Insert Query: {}", query);

        let t_total = std::time::Instant::now();

        let use_tx = self.supports_transactions();
        if use_tx {
            conn.set_autocommit(false)?;
        }

        // Try columnar array binding first (single SQLExecute for all rows).
        // Some drivers (e.g. MS Access) silently ignore SQL_ATTR_PARAMSET_SIZE,
        // so we check row_count after execute and fall back to row-by-row if needed.
        let mut descriptions: Vec<BindParamDesc> = Vec::with_capacity(num_cols);
        descriptions.push(BindParamDesc::i32(false));
        descriptions.push(BindParamDesc::f64(false));
        for _ in input_col_names {
            descriptions.push(BindParamDesc::f64(false));
        }

        // Excel's driver mishandles prepared statements on freshly-created
        // tables (it treats unknown column identifiers as implicit parameter
        // markers). Skip the columnar path entirely for such dialects and use
        // direct-SQL literal inserts instead.
        let columnar_ok: Result<bool, TrnSysError> = if !self.supports_transactions() {
            Ok(false)
        } else {
            (|| -> Result<bool, TrnSysError> {
            let mut prepared = conn.prepare(&query)?;
            {
                let mut inserter = prepared.column_inserter(n, descriptions)?;
                inserter.set_num_rows(n);

                // Fill variant_id column (col 0) — broadcast single i32 value
                {
                    let col = inserter
                        .column_mut(0)
                        .as_slice::<i32>()
                        .expect("variant_id column must be i32");
                    for i in 0..n {
                        col[i] = variant_id;
                    }
                }
                // Fill SimTime column (col 1)
                {
                    let col = inserter
                        .column_mut(1)
                        .as_slice::<f64>()
                        .expect("simtime column must be f64");
                    col.copy_from_slice(sim_times);
                }
                // Fill input data columns (cols 2..N)
                for (col_idx, _) in input_col_names.iter().enumerate() {
                    let col = inserter
                        .column_mut(2 + col_idx)
                        .as_slice::<f64>()
                        .expect("input column must be f64");
                    for (row_idx, row) in input_rows.iter().enumerate() {
                        col[row_idx] = row[col_idx];
                    }
                }

                let t = std::time::Instant::now();
                inserter.execute()?;
                debug!("[timer] columnar execute ({} rows, 1 call): {:?}", n, t.elapsed());
            } // inserter dropped — releases borrow on prepared

            let rows_affected = prepared.row_count()?.unwrap_or(0);
            debug!("[timer] columnar rows_affected: {} (expected {})", rows_affected, n);
            Ok(rows_affected == n)
            })()
        };

        match columnar_ok {
            Ok(true) => {
                // Columnar insert succeeded — commit
                if use_tx {
                    conn.commit()?;
                }
            }
            _ => {
                if use_tx {
                    conn.rollback()?;
                }

                // Build each row as a comma-separated literal value string.
                let mut value_rows: Vec<String> = Vec::with_capacity(n);
                for row_idx in 0..n {
                    let mut vals = Vec::with_capacity(num_cols);
                    vals.push(format!("{}", variant_id));
                    vals.push(format!("{}", sim_times[row_idx]));
                    for col_idx in 0..input_col_names.len() {
                        vals.push(format!("{}", input_rows[row_idx][col_idx]));
                    }
                    value_rows.push(vals.join(", "));
                }

                let table_ref = self.format_data_table(table);
                let result = if self.supports_multi_row_insert() {
                    // Multi-row VALUES: INSERT INTO t (cols) VALUES (...), (...), ...
                    info!("Columnar insert incomplete — falling back to multi-row VALUES");
                    let chunk_size = self.max_rows_per_multi_insert();
                    (|| -> Result<(), TrnSysError> {
                        for (i, chunk) in value_rows.chunks(chunk_size).enumerate() {
                            let body = self.format_multi_row_insert_body(chunk);
                            let sql = format!(
                                "INSERT INTO {} ({}) {}",
                                table_ref, col_name_field, body
                            );
                            if i == 0 {
                                debug!("[multi-row] first SQL ({} chars): {}", sql.len(), &sql[..sql.len().min(500)]);
                            }
                            let t = std::time::Instant::now();
                            conn.execute(&sql, (), None)?;
                            debug!("[multi-row] chunk {} ({} rows): {:?}", i, chunk.len(), t.elapsed());
                        }
                        Ok(())
                    })()
                } else {
                    // Single-row literal INSERT via SQLExecDirect — no parameter binding
                    // overhead (saves N×num_cols SQLBindParameter calls).
                    info!("Columnar insert incomplete — falling back to literal INSERT per row");
                    let insert_prefix = format!(
                        "INSERT INTO {} ({}) VALUES ",
                        table_ref, col_name_field
                    );
                    (|| -> Result<(), TrnSysError> {
                        for row in &value_rows {
                            let sql = format!("{}({})", insert_prefix, row);
                            conn.execute(&sql, (), None)?;
                        }
                        Ok(())
                    })()
                };
                if use_tx {
                    if result.is_err() {
                        let _ = conn.rollback();
                    } else {
                        conn.commit()?;
                    }
                }
                result?;
            }
        }
        if use_tx {
            conn.set_autocommit(true)?;
        }

        debug!(
            "[timer] columnar_batch_insert total ({} rows): {:?}",
            n,
            t_total.elapsed()
        );
        Ok(())
    }

    fn insert_data(
        &self,
        table: &str,
        cols: Vec<(String, Box<dyn InputParameter>)>,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;

        let col_names = cols
            .iter()
            .map(|(name, _)| self.format_identifier(name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, col_names, placeholders
        );
        debug!("Insert Query: {}", query);
        let mut statement = conn.prepare(&query)?;
        let params = cols.into_iter().map(|(_, param)| param).collect::<Vec<_>>();
        statement.execute(params.as_slice())?;

        Ok(())
    }

    fn batch_insert_data(
        &self,
        table: &str,
        col_names: Vec<String>,
        rows: Vec<Vec<Box<dyn InputParameter>>>,
    ) -> Result<(), TrnSysError> {
        if rows.is_empty() {
            return Ok(());
        }
        let conn = self.get_connection()?;

        let col_name_field = col_names
            .iter()
            .map(|name| self.format_identifier(name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (0..col_names.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, col_name_field, placeholders
        );
        debug!("Insert Query: {}", query);

        conn.set_autocommit(false)?;
        let result = (|| -> Result<(), TrnSysError> {
            let mut statement = conn.prepare(&query)?;
            for row in rows {
                statement.execute(row.as_slice())?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = conn.rollback();
        } else {
            conn.commit()?;
        }
        conn.set_autocommit(true)?;
        result
    }

    /// Atomically replace all rows for a variant and insert the given rows.
    fn replace_variant_rows(
        &self,
        table: &str,
        variant_col: &str,
        variant_value: &str,
        col_names: Vec<String>,
        rows: Vec<Vec<Box<dyn InputParameter>>>,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;

        let del = format!(
            "DELETE FROM {} WHERE {} = '{}'",
            table,
            self.format_identifier(variant_col),
            variant_value.replace("'", "''")
        );
        let col_name_field = col_names
            .iter()
            .map(|name| self.format_identifier(name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (0..col_names.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, col_name_field, placeholders
        );

        conn.set_autocommit(false)?;
        let result = (|| -> Result<(), TrnSysError> {
            info!("Replace-Variant: {}", del);
            conn.execute(&del, (), None)?;
            let mut statement = conn.prepare(&query)?;
            for row in rows {
                statement.execute(row.as_slice())?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = conn.rollback();
        } else {
            conn.commit()?;
        }
        conn.set_autocommit(true)?;
        result
    }

    fn query_data(
        &self,
        table: &str,
        cols: Vec<String>,
        additional_conditions: Option<String>,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;
        let col_names = cols
            .iter()
            .map(|str| self.format_identifier(str))
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = format!("SELECT {} FROM {}", col_names, table);
        if let Some(additional) = additional_conditions {
            query.push_str(" ");
            query.push_str(&additional);
        }
        let cursor = conn.execute(&query, (), None)?;

        if let Some(mut cursor) = cursor {
            let headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            println!("Headline: {:?}", headline);

            let data_types = (1..cursor.num_result_cols()? + 1)
                .map(|i| cursor.col_data_type(i as u16))
                .collect::<Result<Vec<_>, _>>()?;

            while let Some(mut row) = cursor.next_row()? {
                for (i, data_type) in data_types.iter().enumerate() {
                    if data_type.is_text_like() {
                        let mut buf = Vec::new();
                        if row.get_text(i as u16 + 1, &mut buf)? {
                            let data = String::from_utf8(buf).unwrap();
                            print!("{},\t", data);
                        } else {
                            print!("NULL,\t");
                        }
                    } else if data_type.is_numeric() {
                        let mut data: f64 = 0f64;
                        if row.get_data(i as u16 + 1, &mut data).is_ok() {
                            print!("{:.2},\t", data);
                        } else {
                            print!("NULL,\t");
                        }
                    } else if data_type.is_date_time() {
                        let mut str_time = "Unknown Datetime".to_string();
                        match data_type {
                            DataType::Date => {
                                let mut data: Date = Date::default();

                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{Y}.{M}.{D}",
                                        Y = data.year,
                                        M = data.month,
                                        D = data.day
                                    );
                                }
                            }
                            DataType::Time { .. } => {
                                let mut data: Time = Time::default();
                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{H}:{M}:{S}",
                                        H = data.hour,
                                        M = data.minute,
                                        S = data.second
                                    );
                                }
                            }
                            DataType::Timestamp { .. } => {
                                let mut data: Timestamp = Timestamp::default();
                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{Y}.{M}.{D} {Hr}:{Min}:{Sec}",
                                        Y = data.year,
                                        M = data.month,
                                        D = data.day,
                                        Hr = data.hour,
                                        Min = data.minute,
                                        Sec = data.second
                                    );
                                }
                            }
                            _ => {}
                        }
                        print!("{},\t", str_time);
                    } else {
                        let mut buf = Vec::new();
                        if row.get_text(i as u16 + 1, &mut buf)? {
                            let data = String::from_utf8(buf).unwrap();
                            print!("{},\t", data);
                        }
                    }
                }
                print!("\n");
            }
        } else {
            println!("No data found.");
        }
        Ok(())
    }
}

pub(crate) trait FileDbProvider<'c>: OdbcProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError>;

    fn get_driver_name(&self) -> String;

    fn setup_by_path(
        &mut self,
        environment: &'c Environment,
        db_path: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let db_path_str = clean_and_ensure_path(db_path)?;
        debug!("DB Path: {:?}", db_path_str);
        self.ensure_file_exists(&db_path_str)?;
        let driver_name = self.get_driver_name();
        let connection_string = format!("Driver={{{}}};DBQ={};", driver_name, &db_path_str);
        self.setup_by_conn_str(environment, &connection_string, conn_options)?;
        Ok(())
    }

    fn ensure_file_exists(&self, db_path: &str) -> Result<(), TrnSysError> {
        let file_exists = fs::exists(db_path)?;
        if !file_exists {
            info!("Creating file: {}", db_path);
            self.get_template()?.create_file(db_path)?;
        } else {
            info!("File already exists: {}", db_path);
        }
        Ok(())
    }
}

pub struct OdbcProviderImpl<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for OdbcProviderImpl<'_> {}

impl_odbc_provider!(OdbcProviderImpl);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ms_access::MsAccessProvider;

    use fs;

    use std::sync::LazyLock;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_create_connection() {
        let db_path = "E:\\test.accdb";
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
        {
            let env = crate::database::tests::ENVIRONMENT.clone();
            let env = env.lock().unwrap();

            let mut ms_access = MsAccessProvider::new();
            ms_access.setup_by_path(&env, db_path, None).unwrap();
            assert!(ms_access.get_connection().is_ok());
        }

        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }
}
