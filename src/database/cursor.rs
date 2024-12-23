use crate::trnsys::error::TrnSysError;
use odbc_api::Cursor;
pub(crate) trait CursorQuery {
    fn find_col_index(&mut self, col_name: &str) -> Result<Option<usize>, TrnSysError>;
}

impl<T> CursorQuery for T
where
    T: Cursor,
{
    fn find_col_index(&mut self, col_name: &str) -> Result<Option<usize>, TrnSysError> {
        let mut table_name_index = None;
        for (index, name) in self.column_names()?.enumerate() {
            let name = name?;
            if name == "TABLE_NAME" {
                table_name_index = Some(index);
                break;
            }
        }
        Ok(table_name_index)
    }
}
