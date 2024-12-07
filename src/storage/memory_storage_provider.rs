use crate::storage::StoreProvider;

pub struct MemoryStorageProvider<T>
where
    T: Clone + Send + Sync,
{
    history: Vec<Option<T>>,
}

impl<T> MemoryStorageProvider<T>
where
    T: Clone + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            history: vec![None],
        }
    }
}

impl<T> StoreProvider<T> for MemoryStorageProvider<T>
where
    T: Clone + Send + Sync,
{
    fn get_history_mut(&mut self) -> &mut Vec<Option<T>> {
        &mut self.history
    }

    fn get_history(&self) -> &Vec<Option<T>> {
        &self.history
    }

    fn set_dyn(&mut self, value: Option<T>) {
        let ref_value = self
            .history
            .last_mut()
            .expect("No history found. Please report this bug.");
        *ref_value = value;
    }

    fn get(&self) -> Option<&T> {
        self.history.last().and_then(|v| v.as_ref())
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        self.history.last_mut().and_then(|v| v.as_mut())
    }

    fn tick(&mut self) {
        self.history.push(None)
    }
}
