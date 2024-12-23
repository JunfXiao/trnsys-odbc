use super::odbc::{FileDbProvider, OdbcProvider};
use super::template::TemplateFile;
use super::*;
use crate::impl_odbc_provider;
use odbc_api::Connection;
use std::sync::Mutex;

pub struct MsAccessProvider<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl_odbc_provider!(MsAccessProvider);
//
// impl<'c> OdbcProvider<'c> for MsAccessProvider<'c> {
//     fn set_connection(&mut self, connection: Connection<'c>) -> Result<(),TrnSysError> {
//         if self.connection.is_some() {
//             return Err(TrnSysError::GeneralError("Connection already set, cannot be set again.".to_string()));
//         }
//         self.connection = Some(Mutex::new(connection));
//         Ok(())
//     }
//
//     fn get_connection(&self) -> Result<&Connection<'c>, TrnSysError> {
//         self.connec&tion.as_ref().ok_or(
//             TrnSysError::GeneralError("DB Connection not set.".to_string())
//         )?.lock()
//             .map_err(|_| TrnSysError::GeneralError("Mutex Lock Error".to_string()))
//             .and_then(|conn| Ok(conn.deref()))
//     }
// }

impl<'c> FileDbProvider<'c> for MsAccessProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError> {
        Ok(TemplateFile::MsAccess)
    }

    fn get_driver_name(&self) -> String {
        "Microsoft Access Driver (*.mdb, *.accdb)".to_string()
    }
}
