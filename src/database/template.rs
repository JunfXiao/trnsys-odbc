use crate::trnsys::error::TrnSysError;
use include_dir::{include_dir, Dir};
use std::io::Write;

static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/db_template");

pub enum TemplateFile {
    MsAccess,
    Sqlite,
    MsExcel,
}

impl TemplateFile {
    pub fn to_binary(&self) -> &'static [u8] {
        match self {
            TemplateFile::MsAccess => TEMPLATE_DIR.get_file("ms-access.accdb").unwrap().contents(),
            TemplateFile::Sqlite => b"",
            TemplateFile::MsExcel => TEMPLATE_DIR.get_file("ms-excel.xlsx").unwrap().contents(),
        }
    }

    pub fn create_file(&self, db_path: &str) -> Result<(), TrnSysError> {
        let mut file = std::fs::File::create(db_path)?;
        file.write_all(self.to_binary())?;
        Ok(())
    }
}
