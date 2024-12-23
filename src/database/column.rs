use strum_macros::EnumIter;

#[derive(Debug, EnumIter, Eq, Hash, PartialEq)]
pub enum MetaCol {
    SimulationTime,
}

impl MetaCol {
    pub fn as_str(&self) -> &'static str {
        match self {
            // MetaCols::CreatedAt => "created_at",
            _ => "SimTime",
        }
    }

    pub fn sql_type(&self) -> &'static str {
        match self {
            // MetaCols::CreatedAt => "DATETIME DEFAULT Now()",
            _ => "DOUBLE",
        }
    }
}

pub fn escape_col_name(col_name: &str) -> String {
    format!("[{}]", col_name.replace("]", "]]"))
}
