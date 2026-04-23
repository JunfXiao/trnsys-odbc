use crate::database::datatype::{ColDataType, ColDef};
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, Eq, Hash, PartialEq, Clone, Ord, PartialOrd)]
pub enum MetaCol {
    VariantId,
    SimulationTime,
}

impl MetaCol {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetaCol::SimulationTime => "SimTime",
            MetaCol::VariantId => "variant_id",
        }
    }
    pub fn col_def(&self) -> ColDef {
        match self {
            MetaCol::SimulationTime => ColDef::new(
                "SimTime",
                ColDataType::Number { decimal: true },
                true,
                false,
            ),
            MetaCol::VariantId => ColDef::new(
                "variant_id",
                ColDataType::Number { decimal: false },
                true,
                false,
            ),
        }
    }
}
