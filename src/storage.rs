use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

type InnerStorage = RwLock<HashMap<String, String>>;

pub struct Storage(InnerStorage);

impl Storage {
    pub fn new() -> Arc<Storage> {
        Arc::new(Storage(RwLock::new(HashMap::new())))
    }

    pub fn insert(&mut self, key: &str, value: &str) ->Result<(), Box<dyn Error + '_>> {
        let mut store = self.0.write()?;
        store.insert(key.to_string(), value.to_string());

        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<String, Box<dyn Error + '_>> {
        let store = self.0.read()?;

        store
            .get(key).cloned()
            .ok_or(format!("Invalid key {key}").into())
    }
}
