use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::database::dialect::SqlDialect;
use crate::database::path::clean_and_ensure_path;
use crate::impl_odbc_provider;
use odbc_api::{Connection, ConnectionOptions, Environment};
use std::sync::Mutex;
use tracing::info;

pub struct MsExcelProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl SqlDialect for MsExcelProvider<'_> {
    fn get_integer_type(&self) -> String {
        "NUMBER".to_string()
    }
    fn get_decimal_type(&self) -> String {
        "NUMBER".to_string()
    }

    fn support_nullability(&self) -> bool {
        false
    }

    fn support_primary_key(&self) -> bool {
        false
    }
}

impl_odbc_provider!(MsExcelProvider);

impl<'c> FileDbProvider<'c> for MsExcelProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError> {
        Ok(TemplateFile::MsExcel)
    }

    fn get_driver_name(&self) -> String {
        "Microsoft Excel Driver (*.xls, *.xlsx, *.xlsm, *.xlsb)".to_string()
    }

    fn setup_by_path(
        &mut self,
        environment: &'c Environment,
        db_path: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let db_path = clean_and_ensure_path(db_path)?;
        info!("DB Path: {}", db_path);
        self.ensure_file_exists(&db_path)?;
        let conn = environment.connect_with_connection_string(
            &format!(
                "DRIVER={};DBQ={};FIRSTROWHASNAMES=1;READONLY=FALSE;",
                self.get_driver_name(),
                db_path
            ),
            conn_options.unwrap_or_default(),
        )?;
        self.set_connection(conn)
    }
}
