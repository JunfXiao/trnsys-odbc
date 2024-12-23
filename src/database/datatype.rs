use odbc_api::DataType;

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
