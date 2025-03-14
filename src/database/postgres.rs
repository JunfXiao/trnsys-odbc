use super::odbc::OdbcProvider;
use super::*;
use crate::database::dialect::SqlDialect;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct PostgreSQLProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for PostgreSQLProvider<'_> {
    // Override decimal type to use PostgreSQL's DOUBLE PRECISION
    fn get_decimal_type(&self) -> String {
        "DOUBLE PRECISION".to_string()
    }

    // PostgreSQL uses TIMESTAMP rather than DATETIME
    fn get_datetime_type(&self) -> String {
        "TIMESTAMP".to_string()
    }

    // PostgreSQL uses double quotes for identifiers, not square brackets
    fn format_identifier(&self, identifier: &str) -> String {
        format!("\"{}\"", identifier.replace("\"", "\"\""))
    }
}

impl_odbc_provider!(PostgreSQLProvider);
