pub mod memory_storage_provider;

/// A trait that defines the interface for a storage provider.
///
/// This trait is used to manage the storage of values over time, providing
/// methods to get, set, and manipulate stored values.
pub trait StoreProvider<T>: Sync + Send {
    /// Returns a mutable reference to the history of stored values.
    fn get_history_mut(&mut self) -> &mut Vec<Option<T>>;

    /// Returns a reference to the history of stored values.
    fn get_history(&self) -> &Vec<Option<T>>;

    /// Sets the current dynamic value.
    fn set_dyn(&mut self, value: Option<T>);

    /// Returns a reference to the current value, if any.
    fn get(&self) -> Option<&T>;

    /// Returns a mutable reference to the current value, if any.
    fn get_mut(&mut self) -> Option<&mut T>;

    /// Returns a mutable reference to the current value, setting it to the
    /// provided default if it is not already set.
    fn get_dyn_mut_or_default(&mut self, default: T) -> &mut T {
        if !self.has_value() {
            self.set_dyn(Some(default));
        }
        self.get_mut().unwrap()
    }

    /// Returns true if a value is currently set.
    fn has_value(&self) -> bool {
        self.get().is_some()
    }

    /// Returns a reference to the last value in the history, if any.
    fn get_last_value(&self) -> Option<&T> {
        if let Some(v) = self.get_history().last() {
            v.as_ref()
        } else {
            None
        }
    }

    /// Advances the storage to the next time step.
    fn tick(&mut self);
}
