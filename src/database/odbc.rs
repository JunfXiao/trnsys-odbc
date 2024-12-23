use super::column::{escape_col_name, MetaCol};
use super::cursor::CursorQuery;
use super::template::TemplateFile;
use crate::database::datatype::DataTypeQuery;
use crate::impl_odbc_provider;
use crate::trnsys::error::TrnSysError;
use indexmap::IndexMap;
use odbc_api::parameter::InputParameter;
use odbc_api::sys::{Date, Time, Timestamp};
use odbc_api::{Connection, ConnectionOptions, Cursor, DataType, Environment, ResultSetMetadata};
use std::fs;
use std::sync::{Mutex, MutexGuard};
use strum::IntoEnumIterator;
use tracing::{debug, info};

pub trait OdbcProvider<'c>: Send + Sync {
    fn set_connection(&mut self, connection: Connection<'c>) -> Result<(), TrnSysError>;
    fn setup_by_conn_str(
        &mut self,
        environment: &'c Environment,
        conn_str: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let connection = environment
            .connect_with_connection_string(conn_str, conn_options.unwrap_or_default())?;
        self.set_connection(connection)?;
        Ok(())
    }
    fn setup_by_dsn(
        &mut self,
        environment: &'c Environment,
        dsn: &str,
        user: &str,
        password: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let connection =
            environment.connect(dsn, user, password, conn_options.unwrap_or_default())?;
        self.set_connection(connection)?;
        Ok(())
    }

    fn get_connection(&self) -> Result<MutexGuard<Connection<'c>>, TrnSysError>;

    fn ensure_table(
        &self,
        table_name: &str,
        cols: Vec<(String, String)>,
    ) -> Result<(), TrnSysError> {
        let connection = self.get_connection()?;

        let mut col_type_dict: IndexMap<String, String> = cols.into_iter().collect();

        // Add predefined columns
        for meta_col in MetaCol::iter() {
            col_type_dict.insert(
                meta_col.as_str().to_string(),
                meta_col.sql_type().to_string(),
            );
        }
        debug!("table_name: {}", table_name);
        // Check if table exists
        let mut table_list_cursor = connection.tables("", "", table_name, "TABLE")?;

        let mut table_name_col_index = table_list_cursor.find_col_index("TABLE_NAME")?;

        let table_name_index =
            table_name_col_index.expect("No TABLE_NAME column found in tables_cursor") as u16;

        let mut table_exists = false;
        // iterate tables to find if table exists
        while let Some(mut row) = table_list_cursor.next_row()? {
            let mut buf = Vec::new();
            if row.get_text(table_name_index, &mut buf)? {
                let name = String::from_utf8(buf).unwrap();
                if name == table_name {
                    table_exists = true;
                    break;
                }
            }
        }

        debug!("Table exists: {}", table_exists);
        if table_exists {
            // try with cursor
            let mut column_info_cursor = connection.columns("", "", table_name, "")?;

            // The returned cursor has the columns: TABLE_CAT, TABLE_SCHEM, TABLE_NAME,
            // COLUMN_NAME, DATA_TYPE, TYPE_NAME, COLUMN_SIZE, BUFFER_LENGTH, DECIMAL_DIGITS,
            // NUM_PREC_RADIX, NULLABLE, REMARKS, COLUMN_DEF, SQL_DATA_TYPE, SQL_DATETIME_SUB, CHAR_OCTET_LENGTH, ORDINAL_POSITION, IS_NULLABLE.
            // Find the index number of the "COLUMN_NAME" column
            let column_name_index = column_info_cursor.find_col_index("COLUMN_NAME")?;
            let data_type_index = column_info_cursor.find_col_index("TYPE_NAME")?.unwrap() as u16;

            if column_name_index.is_none() {
                return Err(TrnSysError::GeneralError(
                    "No COLUMN_NAME column found in columns_cursor".to_string(),
                )
                .into());
            }
            let column_name_index = column_name_index.unwrap() as u16;

            while let Some(mut row) = column_info_cursor.next_row()? {
                let mut buf = Vec::new();
                if row.get_text(column_name_index, &mut buf)? {
                    let column_name = String::from_utf8(buf).unwrap();
                    col_type_dict.shift_remove(&column_name);
                }
            }

            // add missing columns
            for (col, data_type) in col_type_dict {
                let alter_query = format!(
                    "ALTER TABLE {} ADD COLUMN {} {}",
                    table_name,
                    escape_col_name(&col),
                    data_type
                );
                connection.execute(&alter_query, ())?;
            }
        } else {
            // add a new table
            let cols_def = col_type_dict
                .into_iter()
                .map(|(col, data_type)| format!("{} {}", escape_col_name(&col), data_type))
                .collect::<Vec<_>>()
                .join(", \n");

            let create_table_query = format!(
                r"
            CREATE TABLE {}
            (
            id AUTOINCREMENT PRIMARY KEY,
            {}
            );",
                table_name, cols_def
            );
            debug!("Create Table Query: {}", create_table_query);
            connection.execute(&create_table_query, ())?;
        }

        Ok(())
    }

    fn insert_data(
        &self,
        table: &str,
        cols: Vec<(String, Box<dyn InputParameter>)>,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;

        let col_names = cols
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, col_names, placeholders
        );
        let mut statement = conn.prepare(&query)?;
        let mut params = cols.into_iter().map(|(_, param)| param).collect::<Vec<_>>();
        statement.execute(params.as_slice())?;
        Ok(())
    }

    /// Inserts multiple rows of data into the specified table in bulk.
    ///
    /// # Arguments
    ///
    /// * `table` - The name of the table to insert data into.
    /// * `cols` - A vector of column names corresponding to the data being inserted.
    /// * `buffer_desc` - A vector of `BufferDesc` describing the data types and sizes of the columns.
    /// * `data` - A vector of vectors, where each inner vector represents a row of data to be inserted.
    ///            The data is organized as `Vec<Col<Row<data>>>`.
    ///
    /// # Returns
    ///
    /// * `Result<(), TrnSysError>` - Returns `Ok(())` if the data is successfully inserted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an issue with the database connection or if the
    /// data insertion fails.
    // fn bulk_insert_data(&self, table: &str, cols: Vec<String>, buffer_desc: Vec<BufferDesc>, data: Vec<Vec<Box<dyn InputParameter>>>) -> Result<(), TrnSysError> {
    //         let conn = self.get_connection()?;
    //         let col_names = cols.iter().map(|str|escape_col_name(str)).collect::<Vec<_>>().join(", ");
    //         let placeholders = (0..cols.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
    //         let query = format!("INSERT INTO {} ({}) VALUES ({})", table, col_names, placeholders);
    //
    //         let prepared = conn.prepare(&query)?;
    //         let capacity = data.len();
    //         let mut prebound = prepared.into_column_inserter(capacity, buffer_desc.as_slice())?;
    //
    //         prebound.set_num_rows(capacity);
    //         for (i, col_data) in data.iter().enumerate() {
    //             let mut col = prebound.column_mut(i);
    //
    //             for (j, param) in col_data.iter().enumerate() {
    //                 todo!()
    //             }
    //         }
    //
    //         let mut statement = conn.prepare(&query)?;
    //         for row in data {
    //             let params = row.into_iter().collect::<Vec<_>>();
    //             statement.execute(params.as_slice())?;
    //         }
    //         Ok(())
    //     }

    fn query_data(
        &self,
        table: &str,
        cols: Vec<String>,
        additional_conditions: Option<String>,
    ) -> Result<(), TrnSysError> {
        let conn = self.get_connection()?;
        let col_names = cols
            .iter()
            .map(|str| escape_col_name(str))
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = format!("SELECT {} FROM {}", col_names, table);
        if let Some(additional) = additional_conditions {
            query.push_str(" ");
            query.push_str(&additional);
        }
        let cursor = conn.execute(&query, ())?;

        if let Some(mut cursor) = cursor {
            let mut headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            println!("Headline: {:?}", headline);

            let data_types = (1..cursor.num_result_cols()? + 1)
                .map(|i| cursor.col_data_type(i as u16))
                .collect::<Result<Vec<_>, _>>()?;

            while let Some(mut row) = cursor.next_row()? {
                for (i, data_type) in data_types.iter().enumerate() {
                    if data_type.is_text_like() {
                        let mut buf = Vec::new();
                        if row.get_text(i as u16 + 1, &mut buf)? {
                            let data = String::from_utf8(buf).unwrap();
                            print!("{},\t", data);
                        } else {
                            print!("NULL,\t");
                        }
                    } else if data_type.is_numeric() {
                        let mut data: f64 = 0f64;
                        if row.get_data(i as u16 + 1, &mut data).is_ok() {
                            print!("{:.2},\t", data);
                        } else {
                            print!("NULL,\t");
                        }
                    } else if data_type.is_date_time() {
                        let mut str_time = "Unknown Datetime".to_string();
                        match data_type {
                            DataType::Date => {
                                let mut data: Date = Date::default();

                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{Y}.{M}.{D}",
                                        Y = data.year,
                                        M = data.month,
                                        D = data.day
                                    );
                                }
                            }
                            DataType::Time { .. } => {
                                let mut data: Time = Time::default();
                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{H}:{M}:{S}",
                                        H = data.hour,
                                        M = data.minute,
                                        S = data.second
                                    );
                                }
                            }
                            DataType::Timestamp { .. } => {
                                let mut data: Timestamp = Timestamp::default();
                                if row.get_data(i as u16 + 1, &mut data).is_ok() {
                                    str_time = format!(
                                        "{Y}.{M}.{D} {Hr}:{Min}:{Sec}",
                                        Y = data.year,
                                        M = data.month,
                                        D = data.day,
                                        Hr = data.hour,
                                        Min = data.minute,
                                        Sec = data.second
                                    );
                                }
                            }
                            _ => {}
                        }
                        print!("{},\t", str_time);
                    } else {
                        let mut buf = Vec::new();
                        if row.get_text(i as u16 + 1, &mut buf)? {
                            let data = String::from_utf8(buf).unwrap();
                            print!("{},\t", data);
                        }
                    }
                }
                print!("\n");
            }
        } else {
            println!("No data found.");
        }
        Ok(())
    }
}

pub(crate) trait FileDbProvider<'c>: OdbcProvider<'c> {
    fn get_template(&self) -> Result<TemplateFile, TrnSysError>;

    fn get_driver_name(&self) -> String;

    fn setup_by_path(
        &mut self,
        environment: &'c Environment,
        db_path: &str,
        conn_options: Option<ConnectionOptions>,
    ) -> Result<(), TrnSysError>
    where
        Self: Sized,
    {
        let db_path = std::path::absolute(db_path)?;
        let db_path_str = db_path.to_str().unwrap().to_owned();
        fs::create_dir_all(db_path.parent().unwrap())?;
        info!("DB Path: {:?}", db_path);
        self.ensure_file_exists(&db_path_str)?;
        let driver_name = self.get_driver_name();
        let connection_string = format!("Driver={{{}}};DBQ={};", driver_name, &db_path_str);
        let connection = environment
            .connect_with_connection_string(&connection_string, conn_options.unwrap_or_default())?;
        self.set_connection(connection)?;
        Ok(())
    }

    fn ensure_file_exists(&self, db_path: &str) -> Result<(), TrnSysError> {
        if !fs::metadata(db_path).is_ok() {
            self.get_template()?.create_file(db_path)?;
        }
        Ok(())
    }
}

pub struct OdbcProviderImpl<'c> {
    connection: Option<Mutex<Connection<'c>>>,
}

impl_odbc_provider!(OdbcProviderImpl);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ms_access::MsAccessProvider;
    use crate::database::ms_excel::MsExcelProvider;
    use fs;
    use odbc_api::IntoParameter;
    use std::sync::LazyLock;

    static ENVIRONMENT: LazyLock<Environment> = LazyLock::new(|| Environment::new().unwrap());

    #[test]
    fn test_create_connection() {
        let db_path = "E:\\test.accdb";
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
        {
            let mut ms_access = MsAccessProvider::new();
            ms_access
                .setup_by_path(&ENVIRONMENT, db_path, None)
                .unwrap();
            assert!(ms_access.get_connection().is_ok());
        }

        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    #[test]
    fn test_msaccess_insert_query_data() {
        let db_path = "E:\\test2.accdb";
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
        {
            let mut ms_access = MsAccessProvider::new();
            ms_access
                .setup_by_path(&ENVIRONMENT, db_path, None)
                .unwrap();

            // ensure table
            let cols = vec![
                ("UID".to_string(), "INTEGER NULL".to_string()),
                ("Name".to_string(), "TEXT NULL".to_string()),
            ];

            ms_access
                .ensure_table("TestTable", cols)
                .expect("Error Ensuring Table");

            ms_access
                .insert_data(
                    "TestTable",
                    vec![
                        ("UID".to_string(), Box::new(1.into_parameter())),
                        ("Name".to_string(), Box::new("Alice".into_parameter())),
                    ],
                )
                .expect("Error Inserting Data");

            ms_access
                .insert_data(
                    "TestTable",
                    vec![("UID".to_string(), Box::new(2.into_parameter()))],
                )
                .expect("Error Inserting Data without Name");

            ms_access
                .insert_data(
                    "TestTable",
                    vec![("Name".to_string(), Box::new("Bob".into_parameter()))],
                )
                .expect("Error Querying Data");

            ms_access
                .query_data(
                    "TestTable",
                    vec!["UID".to_string(), "Name".to_string()],
                    None,
                )
                .expect("Error Querying Data");
        }
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }

    #[test]
    fn test_msexcel_insert_query_data() {
        let db_path = "E:\\test1.xlsx";
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
        {
            let mut ms_access = MsExcelProvider::new();
            ms_access
                .setup_by_path(&ENVIRONMENT, db_path, None)
                .unwrap();

            // ensure table
            let cols = vec![
                ("UID".to_string(), "INTEGER".to_string()),
                ("Name".to_string(), "TEXT".to_string()),
            ];

            ms_access
                .ensure_table("TestTable", cols)
                .expect("Error Ensuring Table");
            println!("Table Created");
            ms_access
                .insert_data(
                    "TestTable",
                    vec![
                        ("UID".to_string(), Box::new(1.into_parameter())),
                        ("Name".to_string(), Box::new("Alice".into_parameter())),
                    ],
                )
                .expect("Error Inserting Data");

            ms_access
                .insert_data(
                    "TestTable",
                    vec![("UID".to_string(), Box::new(2.into_parameter()))],
                )
                .expect("Error Inserting Data without Name");

            ms_access
                .insert_data(
                    "TestTable",
                    vec![("Name".to_string(), Box::new("Bob".into_parameter()))],
                )
                .expect("Error Querying Data");

            ms_access
                .query_data(
                    "TestTable",
                    vec!["UID".to_string(), "Name".to_string()],
                    None,
                )
                .expect("Error Querying Data");
        }
        if fs::metadata(db_path).is_ok() {
            fs::remove_file(db_path).unwrap();
        }
    }
}
