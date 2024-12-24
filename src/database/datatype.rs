use odbc_api::DataType;
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ColDataType {
    Text,
    Number { decimal: bool },
    DateTime,
}

#[derive(Eq, Debug, Clone)]
pub struct ColDef {
    pub name: String,
    pub data_type: ColDataType,
    pub not_null: bool,
    pub primary_key: bool,
}

impl PartialEq for ColDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for ColDef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl ColDef {
    pub fn new(name: &str, data_type: ColDataType, not_null: bool, primary_key: bool) -> Self {
        ColDef {
            name: name.to_string(),
            data_type,
            not_null,
            primary_key,
        }
    }
}

pub(crate) trait DataTypeQuery {
    fn is_text_like(&self) -> bool;

    fn is_numeric(&self) -> bool;

    fn is_date_time(&self) -> bool;
    fn is_time(&self) -> bool;
    fn is_binary(&self) -> bool;
}

impl DataTypeQuery for DataType {
    fn is_text_like(&self) -> bool {
        match self {
            DataType::Char { .. }
            | DataType::LongVarchar { .. }
            | DataType::Varchar { .. }
            | DataType::WChar { .. }
            | DataType::WVarchar { .. } => true,
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        match self {
            DataType::TinyInt
            | DataType::SmallInt
            | DataType::Integer
            | DataType::BigInt
            | DataType::Real
            | DataType::Float { .. }
            | DataType::Double
            | DataType::Decimal { .. }
            | DataType::Numeric { .. } => true,
            _ => false,
        }
    }

    fn is_date_time(&self) -> bool {
        match self {
            DataType::Date { .. } | DataType::Timestamp { .. } => true,
            _ => false,
        }
    }

    fn is_time(&self) -> bool {
        match self {
            DataType::Time { .. } => true,
            _ => false,
        }
    }

    fn is_binary(&self) -> bool {
        match self {
            DataType::Binary { .. }
            | DataType::Varbinary { .. }
            | DataType::LongVarbinary { .. } => true,
            _ => false,
        }
    }
}
