use crate::database::column::MetaCol;
use odbc_api::parameter::InputParameter;
use odbc_api::IntoParameter;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub struct DataBuffer {
    __private: (),
    pub input_data: Vec<f64>,
    pub meta_cols: Arc<Mutex<BTreeMap<MetaCol, Box<dyn InputParameter>>>>,
}
unsafe impl Send for DataBuffer {}
unsafe impl Sync for DataBuffer {}
impl DataBuffer {
    pub fn new(input_data: Option<Vec<f64>>) -> Self {
        DataBuffer {
            __private: (),
            input_data: input_data.unwrap_or_default(),
            meta_cols: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    fn get_meta_col_keys(&self) -> Vec<MetaCol> {
        self.meta_cols
            .clone()
            .lock()
            .unwrap()
            .keys()
            .map(|k| k.clone())
            .collect::<Vec<_>>()
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

    pub fn get_col_names(&self, input_names: Vec<String>) -> Vec<String> {
        let mut col_names = Vec::new();
        let keys = self.get_meta_col_keys();
        for meta_col in keys {
            col_names.push(meta_col.as_str().to_string());
        }
        col_names
            .into_iter()
            .chain(input_names.into_iter())
            .collect()
    }

    pub fn into_insertable(self) -> Vec<Box<dyn InputParameter>> {
        let keys = self.get_meta_col_keys();
        let mut insertable: Vec<Box<dyn InputParameter>> = Vec::new();
        let mut lock = self.meta_cols.clone();
        let mut meta_cols = lock.lock().unwrap();

        for k in keys {
            let data = meta_cols.remove(&k).unwrap();
            insertable.push(data);
        }
        for (i, data) in self.input_data.iter().enumerate() {
            insertable.push(Box::new(data.into_parameter()));
        }

        insertable
    }
}
