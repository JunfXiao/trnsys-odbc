use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct SqliteProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
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
