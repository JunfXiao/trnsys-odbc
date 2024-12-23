use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::impl_odbc_provider;
use odbc_api::{Connection, ConnectionOptions, Environment};
use std::sync::Mutex;

pub struct MsExcelProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl_odbc_provider!(MsExcelProvider);

impl<'c> FileDbProvider<'c> for MsExcelProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError> {
        Ok(TemplateFile::MsExcel)
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

    fn get_driver_name(&self) -> String {
        "Microsoft Excel Driver (*.xls, *.xlsx, *.xlsm, *.xlsb)".to_string()
    }
}
