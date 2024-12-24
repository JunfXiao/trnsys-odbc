use crate::database::datatype::{ColDataType, ColDef};
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, Eq, Hash, PartialEq, Clone)]
pub enum MetaCol {
    Variant,
    SimulationTime,
}

impl MetaCol {
    pub fn as_str(&self) -> &'static str {
        match self {
            // MetaCols::CreatedAt => "created_at",
            MetaCol::SimulationTime => "SimTime",
            MetaCol::Variant => "Variant",
        }
    }
    pub fn col_def(&self) -> ColDef {
        match self {
            // MetaCols::CreatedAt => "DATETIME DEFAULT Now()",
            MetaCol::SimulationTime => ColDef::new(
                "SimTime",
                ColDataType::Number { decimal: false },
                true,
                true,
            ),
            MetaCol::Variant => ColDef::new("Variant", ColDataType::Text, true, true),
        }
    }
}

pub fn escape_col_name(col_name: &str) -> String {
    format!("[{}]", col_name.replace("]", "]]"))
}
