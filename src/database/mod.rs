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
mod dialect;
mod path;

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

#[cfg(test)]
mod tests {
    use crate::database::datatype::{ColDataType, ColDef};
    use crate::database::ms_access::MsAccessProvider;
    use crate::database::ms_excel::MsExcelProvider;
    use crate::database::odbc::FileDbProvider;
    use crate::database::path::clean_and_ensure_path;
    use crate::database::sqlite::SqliteProvider;
    use odbc_api::{Environment, IntoParameter};
    use std::fs;
    use std::sync::LazyLock;
    use tracing_test::traced_test;

    static ENVIRONMENT: LazyLock<Environment> = LazyLock::new(|| {
        // Initialize ODBC Environment
        Environment::new().unwrap()
    });

    fn test_db(provider: &mut dyn FileDbProvider) {
        // ensure table
        let cols = vec![
            ColDef::new("UID", ColDataType::Number { decimal: false }, false, false),
            ColDef::new("Name", ColDataType::Text, false, false),
        ];

        provider
            .ensure_table("TestTable", cols, None)
            .expect("Error Ensuring Table");

        provider
            .insert_data(
                "TestTable",
                vec![
                    ("SimTime".to_string(), Box::new(1.into_parameter())),
                    ("UID".to_string(), Box::new(1.into_parameter())),
                    ("Name".to_string(), Box::new("Alice".into_parameter())),
                    ("Variant".to_string(), Box::new("Variant1".into_parameter())),
                ],
            )
            .expect("Error Inserting Data");

        provider
            .insert_data(
                "TestTable",
                vec![
                    ("SimTime".to_string(), Box::new(1.into_parameter())),
                    ("UID".to_string(), Box::new(1.into_parameter())),
                    ("Name".to_string(), Box::new("Alice".into_parameter())),
                    ("Variant".to_string(), Box::new("Variant2".into_parameter())),
                ],
            )
            .expect("Error Inserting Data");

        provider
            .insert_data(
                "TestTable",
                vec![
                    ("SimTime".to_string(), Box::new(2.into_parameter())),
                    ("Name".to_string(), Box::new("Bob".into_parameter())),
                    ("Variant".to_string(), Box::new("Variant1".into_parameter())),
                ],
            )
            .expect("Error Querying Data");

        provider
            .query_data(
                "TestTable",
                vec![
                    "Variant".to_string(),
                    "SimTime".to_string(),
                    "UID".to_string(),
                    "Name".to_string(),
                ],
                None,
            )
            .expect("Error Querying Data");
    }

    #[traced_test]
    #[test]
    fn test_ms_access() {
        let path = clean_and_ensure_path("test.accdb").unwrap();
        let db_path = &path;
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }

        {
            let mut provider = MsAccessProvider::new();
            provider.setup_by_path(&ENVIRONMENT, db_path, None).unwrap();

            test_db(&mut provider);
        }

        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    #[traced_test]
    #[test]
    fn test_ms_excel() {
        let path = clean_and_ensure_path("test.xlsx").unwrap();
        let db_path = &path;
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }

        {
            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&ENVIRONMENT, db_path, None).unwrap();

            test_db(&mut provider);
        }
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    #[traced_test]
    #[test]
    fn test_sqlite() {
        let path = clean_and_ensure_path("test.db").unwrap();
        let db_path = &path;
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }

        {
            let mut provider = SqliteProvider::new();
            provider.setup_by_path(&ENVIRONMENT, db_path, None).unwrap();

            test_db(&mut provider);
        }
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }
}
