use crate::database::datatype::{ColDataType, ColDef};

pub trait SqlDialect {
    fn get_text_type(&self) -> String {
        "TEXT".to_string()
    }

    fn get_integer_type(&self) -> String {
        "INTEGER".to_string()
    }

    fn get_decimal_type(&self) -> String {
        "DOUBLE".to_string()
    }

    fn get_datetime_type(&self) -> String {
        "DATETIME".to_string()
    }

    fn support_nullability(&self) -> bool {
        true
    }

    fn support_primary_key(&self) -> bool {
        true
    }

    fn get_col_def_str(&self, col_def: &ColDef) -> String {
        let mut data_type_str = match col_def.data_type {
            ColDataType::Text => self.get_text_type(),
            ColDataType::Number { decimal } => {
                if decimal {
                    self.get_decimal_type()
                } else {
                    self.get_integer_type()
                }
            }
            ColDataType::DateTime => self.get_datetime_type(),
        }
        .to_owned();
        if col_def.not_null && self.support_nullability() {
            data_type_str.push_str(" NOT NULL");
        }
        format!(
            "{} {}",
            self.format_identifier(&col_def.name),
            data_type_str
        )
    }

    fn format_identifier(&self, identifier: &str) -> String {
        format!("[{}]", identifier.replace("]", "]]"))
    }

    fn get_primary_key_str(&self, col_defs: Vec<&ColDef>) -> String {
        if !self.support_primary_key() {
            return "".to_string();
        }
        let primary_cols = col_defs
            .into_iter()
            .filter(|col_def| col_def.primary_key)
            .collect::<Vec<_>>();
        if primary_cols.is_empty() {
            return "".to_string();
        }

        let primary_key_str = primary_cols
            .iter()
            .map(|col_def| self.format_identifier(&col_def.name))
            .collect::<Vec<_>>()
            .join(", ");

        format!("PRIMARY KEY ({})", primary_key_str)
    }
}
