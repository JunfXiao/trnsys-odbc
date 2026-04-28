use crate::database::datatype::{ColDataType, ColDef};

pub trait SqlDialect {
    fn get_text_type(&self) -> String {
        "VARCHAR(255)".to_string()
    }

    fn get_integer_type(&self) -> String {
        "INTEGER".to_string()
    }

    fn get_decimal_type(&self) -> String {
        "FLOAT".to_string()
    }

    fn get_boolean_type(&self) -> String {
        "BIT".to_string()
    }

    fn get_datetime_type(&self) -> String {
        "DATETIME".to_string()
    }

    fn support_nullability(&self) -> bool {
        true
    }

    fn support_primary_key(&self) -> bool {
        true
    }

    /// Whether the dialect supports server-side auto-increment for primary keys.
    /// Excel's ODBC driver does not; callers must generate ids manually.
    fn supports_autoincrement(&self) -> bool {
        true
    }

    /// Whether the driver supports ODBC transactions (autocommit toggling,
    /// commit/rollback). Excel's driver returns "Optional feature not
    /// implemented" for SQLSetConnectAttr(SQL_ATTR_AUTOCOMMIT).
    fn supports_transactions(&self) -> bool {
        true
    }

    /// Whether the driver supports row-level DELETE. Jet/Excel's ISAM treats
    /// its sheets as linked tables and rejects DELETE with
    /// "Deleting data in a linked table is not supported by this ISAM".
    /// Callers fall back to DROP+CREATE to wipe state.
    fn supports_delete(&self) -> bool {
        true
    }

    /// Format a text value as an SQL literal (single-quoted, with single quotes doubled).
    fn format_text_literal(&self, value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    /// Reference a data table in a DML statement. Excel overrides to use the
    /// `[sheet$]` form because its ODBC driver caches named-range metadata at
    /// connection time and won't see a table that was CREATEd on this session.
    fn format_data_table(&self, table_name: &str) -> String {
        self.format_identifier(table_name)
    }

    /// Dialect hint for how many rows to insert per transaction/chunk.
    /// Defaults to 400 and can be overridden by providers (e.g., MS Access).
    fn preferred_insert_chunk_size(&self) -> usize {
        400
    }

    /// SQL expression that evaluates to the current timestamp.
    /// Defaults to the SQL-standard `CURRENT_TIMESTAMP`; MS Access overrides to `Now()`.
    fn current_timestamp_expr(&self) -> &'static str {
        "CURRENT_TIMESTAMP"
    }

    /// Whether the dialect supports multi-row INSERT (e.g. `VALUES (...), (...)`).
    /// Access returns false — it uses individual INSERT with literal values instead.
    fn supports_multi_row_insert(&self) -> bool {
        true
    }

    /// Maximum number of rows per multi-row INSERT statement.
    fn max_rows_per_multi_insert(&self) -> usize {
        1000
    }

    /// Format multiple rows of literal values into an INSERT body.
    /// Each entry in `rows` is a comma-separated value list (e.g. "1, 0.5, 3.14").
    /// Default emits `VALUES (row1), (row2), ...`; Access overrides to `SELECT ... UNION ALL ...`.
    fn format_multi_row_insert_body(&self, rows: &[String]) -> String {
        let values = rows
            .iter()
            .map(|r| format!("({})", r))
            .collect::<Vec<_>>()
            .join(", ");
        format!("VALUES {}", values)
    }

    /// Full column definition for an auto-increment primary key column.
    /// Default uses SQL Server syntax; SQLite and Access override.
    fn autoincrement_pk_def(&self, col_name: &str) -> String {
        format!(
            "{} INTEGER IDENTITY(0, 1) PRIMARY KEY",
            self.format_identifier(col_name)
        )
    }

    fn get_col_def_str(&self, col_def: &ColDef) -> String {
        let mut data_type_str = match col_def.data_type {
            ColDataType::Text => self.get_text_type(),
            ColDataType::Number { decimal } => {
                if decimal {
                    self.get_decimal_type()
                } else {
                    self.get_integer_type()
                }
            }
            ColDataType::Boolean => self.get_boolean_type(),
            ColDataType::DateTime => self.get_datetime_type(),
        }
        .to_owned();
        if col_def.not_null && self.support_nullability() {
            data_type_str.push_str(" NOT NULL");
        }
        format!(
            "{} {}",
            self.format_identifier(&col_def.name),
            data_type_str
        )
    }

    fn format_identifier(&self, identifier: &str) -> String {
        format!("[{}]", identifier.replace("]", "]]"))
    }

    fn get_primary_key_str(&self, col_defs: Vec<&ColDef>) -> String {
        if !self.support_primary_key() {
            return "".to_string();
        }
        let primary_cols = col_defs
            .into_iter()
            .filter(|col_def| col_def.primary_key)
            .collect::<Vec<_>>();
        if primary_cols.is_empty() {
            return "".to_string();
        }

        let primary_key_str = primary_cols
            .iter()
            .map(|col_def| self.format_identifier(&col_def.name))
            .collect::<Vec<_>>()
            .join(", ");

        format!("PRIMARY KEY ({})", primary_key_str)
    }
}
