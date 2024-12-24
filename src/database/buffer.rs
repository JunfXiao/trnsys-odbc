use crate::database::column::MetaCol;
use odbc_api::parameter::InputParameter;
use odbc_api::IntoParameter;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct DataBuffer {
    pub input_data: Vec<f64>,
    pub meta_cols: Arc<Mutex<HashMap<MetaCol, Box<dyn InputParameter>>>>,
}
unsafe impl Send for DataBuffer {}
unsafe impl Sync for DataBuffer {}
impl DataBuffer {
    pub fn new(input_data: Option<Vec<f64>>) -> Self {
        DataBuffer {
            input_data: input_data.unwrap_or_default(),
            meta_cols: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert_meta_col<T>(&mut self, meta_col: MetaCol, data: T)
    where
        T: IntoParameter,
        <T as IntoParameter>::Parameter: 'static,
    {
        let binding = self.meta_cols.clone();
        binding
            .lock()
            .unwrap()
            .insert(meta_col, Box::new(data.into_parameter()));
    }

    pub fn into_insertable(
        self,
        input_data_names: &Vec<String>,
    ) -> Vec<(String, Box<dyn InputParameter>)> {
        let mut insertable: Vec<(String, Box<dyn InputParameter>)> = Vec::new();
        for (meta_col, data) in self.meta_cols.clone().lock().unwrap().drain() {
            insertable.push((meta_col.as_str().to_string(), data));
        }
        for (i, data) in self.input_data.iter().enumerate() {
            insertable.push((input_data_names[i].clone(), Box::new(data.into_parameter())));
        }

        insertable
    }
}
