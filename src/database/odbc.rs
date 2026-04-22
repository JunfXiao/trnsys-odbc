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
        }
        Ok(())
    }

    fn remove_variant(&self, table_name: &str, variant_name: &str) -> Result<(), TrnSysError> {
        let connection = self.get_connection()?;
        let query = format!(
            "DELETE FROM {} WHERE {} = '{}'",
            table_name,
            self.format_identifier(MetaCol::Variant.as_str()),
            variant_name
        );
        info!("Remove Variant Query: {}", query);
        connection.execute(&query, (), None)?;
        info!("Variant removed.");
        Ok(())
    }

    /// Bulk-insert rows using ODBC columnar array parameter binding.
    /// Sends all rows in a single SQLExecute call instead of one per row.
    ///
    /// Column layout (fixed): variant_col (text), simtime_col (f64), input_col_names... (f64).
    fn columnar_batch_insert(
        &self,
        table: &str,
        variant_col: &str,
        simtime_col: &str,
        input_col_names: &[String],
        variant_value: &str,
        sim_times: &[f64],
        input_rows: &[Vec<f64>],
    ) -> Result<(), TrnSysError> {
        let n = sim_times.len();
        if n == 0 {
            return Ok(());
        }
        let conn = self.get_connection()?;

        // Build INSERT query with all column names
        let col_name_field = std::iter::once(variant_col)
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

        conn.set_autocommit(false)?;

        // Try columnar array binding first (single SQLExecute for all rows).
        // Some drivers (e.g. MS Access) silently ignore SQL_ATTR_PARAMSET_SIZE,
        // so we check row_count after execute and fall back to row-by-row if needed.
        let mut descriptions: Vec<BindParamDesc> = Vec::with_capacity(num_cols);
        descriptions.push(BindParamDesc::text(variant_value.len().max(1)));
        descriptions.push(BindParamDesc::f64(false));
        for _ in input_col_names {
            descriptions.push(BindParamDesc::f64(false));
        }

        let columnar_ok = (|| -> Result<bool, TrnSysError> {
            let mut prepared = conn.prepare(&query)?;
            {
                let mut inserter = prepared.column_inserter(n, descriptions)?;
                inserter.set_num_rows(n);

                // Fill Variant column (col 0)
                {
                    let variant_bytes = variant_value.as_bytes();
                    let mut col = inserter
                        .column_mut(0)
                        .as_text_view()
                        .expect("variant column must be text");
                    for i in 0..n {
                        col.set_cell(i, Some(variant_bytes));
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
        })();

        match columnar_ok {
            Ok(true) => {
                // Columnar insert succeeded — commit
                conn.commit()?;
            }
            _ => {
                // Array binding not fully supported — rollback and retry row-by-row
                let _ = conn.rollback();
                info!("Columnar insert incomplete — falling back to row-by-row");

                conn.set_autocommit(false)?;
                let result = (|| -> Result<(), TrnSysError> {
                    let mut statement = conn.prepare(&query)?;
                    for row_idx in 0..n {
                        let mut params: Vec<Box<dyn InputParameter>> =
                            Vec::with_capacity(num_cols);
                        params.push(Box::new(variant_value.to_string().into_parameter()));
                        params.push(Box::new(sim_times[row_idx].into_parameter()));
                        for col_idx in 0..input_col_names.len() {
                            params.push(Box::new(input_rows[row_idx][col_idx].into_parameter()));
                        }
                        statement.execute(params.as_slice())?;
                    }
                    Ok(())
                })();
                if result.is_err() {
                    let _ = conn.rollback();
                } else {
                    conn.commit()?;
                }
                result?;
            }
        }
        conn.set_autocommit(true)?;

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
