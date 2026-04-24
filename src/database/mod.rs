use crate::trnsys::error::*;

pub(super) mod column;
pub mod ms_access;
pub mod ms_excel;
pub mod odbc;
pub mod postgres;
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
    use crate::database::odbc::{FileDbProvider, OdbcProvider};
    use crate::database::path::clean_and_ensure_path;
    use crate::database::sqlite::SqliteProvider;
    use odbc_api::{Cursor, Environment, IntoParameter};
    use std::cell::RefCell;
    use std::fs;
    use std::ops::Deref;
    use std::sync::{Arc, LazyLock, Mutex};
    use tracing_test::traced_test;

    pub static ENVIRONMENT: LazyLock<Arc<Mutex<Environment>>> = LazyLock::new(|| {
        // Initialize ODBC Environment
        Arc::new(Mutex::new(
            Environment::new().expect("Failed to create ODBC Environment"),
        ))
    });

    /// Acquire the shared ODBC environment, tolerating a poisoned mutex so
    /// one failing test does not cascade-fail every later one.
    fn lock_env() -> std::sync::MutexGuard<'static, Environment> {
        match ENVIRONMENT.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        }
    }

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
                    ("variant_id".to_string(), Box::new(1i32.into_parameter())),
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
                    ("variant_id".to_string(), Box::new(2i32.into_parameter())),
                ],
            )
            .expect("Error Inserting Data");

        provider
            .insert_data(
                "TestTable",
                vec![
                    ("SimTime".to_string(), Box::new(2.into_parameter())),
                    ("Name".to_string(), Box::new("Bob".into_parameter())),
                    ("variant_id".to_string(), Box::new(1i32.into_parameter())),
                ],
            )
            .expect("Error Querying Data");

        provider
            .query_data(
                "TestTable",
                vec![
                    "variant_id".to_string(),
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
            let env = lock_env();

            let mut provider = MsAccessProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            test_db(&mut provider);
        }

        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    fn query_all_rows(
        provider: &dyn OdbcProvider,
        table_ref: &str,
        col_count: u16,
    ) -> Vec<Vec<String>> {
        let conn = provider.get_connection().unwrap();
        let select = format!("SELECT * FROM {}", table_ref);
        let mut out = Vec::new();
        if let Some(mut cursor) = conn.execute(&select, (), None).unwrap() {
            while let Some(mut row) = cursor.next_row().unwrap() {
                let mut values = Vec::new();
                for i in 1..=col_count {
                    let mut buf = Vec::new();
                    if row.get_text(i, &mut buf).unwrap() {
                        values.push(String::from_utf8_lossy(&buf).into_owned());
                    } else {
                        values.push("".to_string());
                    }
                }
                out.push(values);
            }
        }
        out
    }

    #[traced_test]
    #[test]
    fn test_ms_excel_preserves_other_variants_on_cleanup() {
        let path = clean_and_ensure_path("test_preserve.xlsx").unwrap();
        let db_path = &path;
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }

        let cols = || {
            vec![
                ColDef::new("x", ColDataType::Number { decimal: true }, false, false),
                ColDef::new("y", ColDataType::Number { decimal: true }, false, false),
            ]
        };

        // Seed: create two variants, each with distinct data rows.
        {
            let env = lock_env();
            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            provider.ensure_variants_table().unwrap();
            let id_a = provider.ensure_variant("A").unwrap();
            let id_b = provider.ensure_variant("B").unwrap();
            provider.ensure_table("Result", cols(), None).unwrap();

            provider
                .columnar_batch_insert(
                    "Result",
                    "variant_id",
                    "SimTime",
                    &["x".to_string(), "y".to_string()],
                    id_a,
                    &[1.0, 2.0],
                    &[vec![0.1, 0.2], vec![0.3, 0.4]],
                )
                .unwrap();
            provider
                .columnar_batch_insert(
                    "Result",
                    "variant_id",
                    "SimTime",
                    &["x".to_string(), "y".to_string()],
                    id_b,
                    &[10.0, 20.0],
                    &[vec![9.1, 9.2], vec![9.3, 9.4]],
                )
                .unwrap();
        }

        // Rerun variant A: should drop A's old rows, write fresh A rows, keep B's rows intact.
        {
            let env = lock_env();
            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            provider.ensure_variants_table().unwrap();
            let id_a = provider.ensure_variant("A").unwrap();
            provider.ensure_table("Result", cols(), None).unwrap();

            // Clear A's rows — B's should survive.
            provider.remove_variant_data("Result", id_a).unwrap();

            // Verify after cleanup: only B's rows remain.
            let rows = query_all_rows(&provider, "[Result]", 4);
            let as_int = |s: &str| s.parse::<f64>().ok().map(|f| f as i32);
            let b_rows: Vec<_> = rows
                .iter()
                .filter(|r| as_int(&r[0]) == Some(2))
                .collect();
            assert_eq!(
                b_rows.len(),
                2,
                "expected 2 rows for variant B to be preserved, got rows: {:?}",
                rows
            );
            let a_rows: Vec<_> = rows.iter().filter(|r| as_int(&r[0]) == Some(1)).collect();
            assert_eq!(
                a_rows.len(),
                0,
                "expected A's rows to be cleared, got: {:?}",
                a_rows
            );

            // Insert fresh A rows.
            provider
                .columnar_batch_insert(
                    "Result",
                    "variant_id",
                    "SimTime",
                    &["x".to_string(), "y".to_string()],
                    id_a,
                    &[5.0],
                    &[vec![0.55, 0.66]],
                )
                .unwrap();

            let rows = query_all_rows(&provider, "[Result]", 4);
            let as_int = |s: &str| s.parse::<f64>().ok().map(|f| f as i32);
            assert_eq!(rows.len(), 3, "expected 2 B rows + 1 new A row; got {:?}", rows);
            let b_rows: Vec<_> = rows.iter().filter(|r| as_int(&r[0]) == Some(2)).collect();
            assert_eq!(b_rows.len(), 2, "B rows should still be present: {:?}", rows);
            let a_rows: Vec<_> = rows.iter().filter(|r| as_int(&r[0]) == Some(1)).collect();
            assert_eq!(a_rows.len(), 1, "should have 1 fresh A row: {:?}", a_rows);
            assert_eq!(
                a_rows[0][1].parse::<f64>().unwrap() as i32,
                5,
                "new A SimTime should be 5"
            );
        }

        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    #[traced_test]
    #[test]
    fn test_ms_excel_variants() {
        let path = clean_and_ensure_path("test_variants.xlsx").unwrap();
        let db_path = &path;
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }

        {
            let env = lock_env();

            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            provider.ensure_variants_table().expect("ensure_variants_table");
            let id1 = provider.ensure_variant("alpha").expect("ensure_variant alpha");
            let id2 = provider.ensure_variant("beta").expect("ensure_variant beta");
            let id1b = provider.ensure_variant("alpha").expect("re-lookup alpha");
            assert_eq!(id1, id1b);
            assert_ne!(id1, id2);

            let cols = vec![
                ColDef::new("x", ColDataType::Number { decimal: true }, false, false),
                ColDef::new("y", ColDataType::Number { decimal: true }, false, false),
            ];
            provider
                .ensure_table("Result", cols, None)
                .expect("ensure_table Result");
            provider
                .remove_variant_data("Result", id1)
                .expect("remove_variant_data");
            provider
                .columnar_batch_insert(
                    "Result",
                    "variant_id",
                    "SimTime",
                    &["x".to_string(), "y".to_string()],
                    id1,
                    &[1.0, 2.0, 3.0],
                    &[vec![0.1, 0.2], vec![0.3, 0.4], vec![0.5, 0.6]],
                )
                .expect("columnar_batch_insert");
            provider
                .mark_variant_complete(id1)
                .expect("mark_variant_complete");
        }

        // Second pass: file already exists — exercises the rerun path where
        // DELETE FROM <data_table> must succeed (Jet rejects DELETE against a
        // named range, so we use the `[sheet$]` form).
        {
            let env = lock_env();

            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            provider.ensure_variants_table().expect("ensure_variants_table rerun");
            let id1 = provider.ensure_variant("alpha").expect("re-ensure alpha");

            let cols = vec![
                ColDef::new("x", ColDataType::Number { decimal: true }, false, false),
                ColDef::new("y", ColDataType::Number { decimal: true }, false, false),
            ];
            provider.ensure_table("Result", cols, None).expect("ensure_table rerun");
            provider
                .remove_variant_data("Result", id1)
                .expect("remove_variant_data rerun");
            provider
                .columnar_batch_insert(
                    "Result",
                    "variant_id",
                    "SimTime",
                    &["x".to_string(), "y".to_string()],
                    id1,
                    &[10.0, 20.0],
                    &[vec![1.1, 1.2], vec![2.1, 2.2]],
                )
                .expect("columnar_batch_insert rerun");
            provider.mark_variant_complete(id1).expect("mark_variant_complete rerun");
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
            let env = lock_env();

            let mut provider = MsExcelProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

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
            let env = lock_env();

            let mut provider = SqliteProvider::new();
            provider.setup_by_path(&*env, db_path, None).unwrap();

            test_db(&mut provider);
        }
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }
}
