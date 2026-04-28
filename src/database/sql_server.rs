use super::odbc::OdbcProvider;
use super::*;
use crate::database::dialect::SqlDialect;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct SqlServerProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for SqlServerProvider<'_> {
    fn get_text_type(&self) -> String {
        "NVARCHAR(255)".to_string()
    }

    fn get_integer_type(&self) -> String {
        "INT".to_string()
    }

    fn get_decimal_type(&self) -> String {
        "FLOAT".to_string()
    }

    fn get_boolean_type(&self) -> String {
        "BIT".to_string()
    }

    fn get_datetime_type(&self) -> String {
        "DATETIME2".to_string()
    }

    fn autoincrement_pk_def(&self, col_name: &str) -> String {
        format!(
            "{} INT IDENTITY(0, 1) PRIMARY KEY",
            self.format_identifier(col_name)
        )
    }

    fn max_rows_per_multi_insert(&self) -> usize {
        1000
    }
}

impl_odbc_provider!(SqlServerProvider);
