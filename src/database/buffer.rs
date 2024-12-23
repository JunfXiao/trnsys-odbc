use crate::database::column::MetaCol;
use odbc_api::parameter::InputParameter;
use odbc_api::IntoParameter;
use std::collections::HashMap;

pub struct DataBuffer {
    pub input_data: Vec<f64>,
    pub meta_cols: HashMap<MetaCol, f64>,
}

impl DataBuffer {
    pub fn new(input_data: Option<Vec<f64>>) -> Self {
        DataBuffer {
            input_data: input_data.unwrap_or_default(),
            meta_cols: HashMap::new(),
        }
    }

    pub fn to_insertable(
        &self,
        input_data_names: &Vec<String>,
    ) -> Vec<(String, Box<dyn InputParameter>)> {
        let mut insertable: Vec<(String, Box<dyn InputParameter>)> = Vec::new();
        for (i, data) in self.input_data.iter().enumerate() {
            insertable.push((input_data_names[i].clone(), Box::new(data.into_parameter())));
        }
        for (meta_col, data) in self.meta_cols.iter() {
            insertable.push((
                meta_col.as_str().to_string(),
                Box::new(data.into_parameter()),
            ));
        }
        insertable
    }
}
