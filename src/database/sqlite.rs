use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::database::dialect::SqlDialect;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct SqliteProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for SqliteProvider<'_> {
    fn get_integer_type(&self) -> String {
        "INTEGER".to_string()
    }
    fn get_decimal_type(&self) -> String {
        "REAL".to_string()
    }
}

impl_odbc_provider!(SqliteProvider);

impl<'c> FileDbProvider<'c> for SqliteProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError> {
        Ok(TemplateFile::Sqlite)
    }

    fn get_driver_name(&self) -> String {
        "SQLite3 ODBC Driver".to_string()
    }
}
