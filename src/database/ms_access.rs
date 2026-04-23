use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::database::dialect::SqlDialect;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct MsAccessProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for MsAccessProvider<'_> {
    fn get_decimal_type(&self) -> String {
        "FLOAT".to_string()
    }

    fn get_boolean_type(&self) -> String {
        "BIT".to_string()
    }

    fn preferred_insert_chunk_size(&self) -> usize {
        200
    }

    fn supports_multi_row_insert(&self) -> bool {
        false
    }

    fn current_timestamp_expr(&self) -> &'static str {
        "Now()"
    }

    fn autoincrement_pk_def(&self, col_name: &str) -> String {
        format!(
            "{} AUTOINCREMENT(0, 1) PRIMARY KEY",
            self.format_identifier(col_name)
        )
    }
}

impl_odbc_provider!(MsAccessProvider);

impl<'c> FileDbProvider<'c> for MsAccessProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError> {
        Ok(TemplateFile::MsAccess)
    }

    fn get_driver_name(&self) -> String {
        "Microsoft Access Driver (*.mdb, *.accdb)".to_string()
    }
}
