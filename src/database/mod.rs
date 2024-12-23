use crate::trnsys::error::*;

pub(super) mod column;
pub mod ms_access;
pub mod ms_excel;
pub mod odbc;
pub mod sqlite;
pub mod template;

pub(super) mod datatype;

pub(super) mod cursor;

pub mod buffer;

#[macro_export]
macro_rules! impl_odbc_provider {
    ($provider:ident) => {
        impl $provider<'_> {
            pub fn new() -> Self {
                $provider { connection: None }
            }
        }

        impl<'c> OdbcProvider<'c> for $provider<'c> {
            fn set_connection(&mut self, connection: Connection<'c>) -> Result<(), TrnSysError> {
                if self.connection.is_some() {
                    return Err(TrnSysError::GeneralError(
                        "Connection already set, cannot be set again.".to_string(),
                    ));
                }
                self.connection = Some(Mutex::new(connection));
                Ok(())
            }

            fn get_connection(&self) -> Result<std::sync::MutexGuard<Connection<'c>>, TrnSysError> {
                Ok(self
                    .connection
                    .as_ref()
                    .ok_or(TrnSysError::GeneralError(
                        "DB Connection not set.".to_string(),
                    ))?
                    .lock()
                    .map_err(|_| TrnSysError::GeneralError("Mutex Lock Error".to_string()))?)
            }
        }
    };
}
