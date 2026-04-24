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

    // Jet/ACE Excel driver rejects "BOOLEAN"; BIT is the accepted literal.
    fn get_boolean_type(&self) -> String {
        "BIT".to_string()
    }

    fn support_nullability(&self) -> bool {
        false
    }

    fn support_primary_key(&self) -> bool {
        false
    }

    fn supports_autoincrement(&self) -> bool {
        false
    }

    // Jet Excel has no CURRENT_TIMESTAMP keyword; use the Jet Now() function.
    fn current_timestamp_expr(&self) -> &'static str {
        "Now()"
    }

    fn supports_transactions(&self) -> bool {
        false
    }

    // Jet (Access/Excel) does not accept multi-row VALUES. Single-row inserts
    // are used via the literal-fallback path in columnar_batch_insert.
    fn supports_multi_row_insert(&self) -> bool {
        false
    }

    // Excel CREATE TABLE rejects PRIMARY KEY and AUTOINCREMENT; emit a plain column.
    fn autoincrement_pk_def(&self, col_name: &str) -> String {
        format!(
            "{} {}",
            self.format_identifier(col_name),
            self.get_integer_type()
        )
    }

    // Use the named-range form `[name]` (created by CREATE TABLE) for DML.
    // The named range carries the declared column types, so INSERTs write
    // numeric values as numbers instead of text (which `[name$]` would do,
    // since it bypasses the range's schema). Requires the table to already
    // exist — which is true for our usage (CREATE TABLE runs first).
    fn format_data_table(&self, table_name: &str) -> String {
        self.format_identifier(table_name)
    }

    // Jet's Excel ISAM rejects DELETE entirely (both named-range and sheet
    // forms). ensure_table falls back to DROP+CREATE when this returns false.
    fn supports_delete(&self) -> bool {
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
        self.setup_by_conn_str(
            environment,
            &format!(
                "DRIVER={{{}}};DBQ={};FIRSTROWHASNAMES=1;READONLY=FALSE;",
                self.get_driver_name(),
                db_path
            ),
            conn_options,
        )
    }
}
