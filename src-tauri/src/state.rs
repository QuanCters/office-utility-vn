use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub task_registry: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Arc::new(Mutex::new(conn)),
            task_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_task(&self, task_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut reg = self.task_registry.lock().unwrap();
        reg.insert(task_id.to_string(), Arc::clone(&flag));
        flag
    }

    pub fn unregister_task(&self, task_id: &str) {
        let mut reg = self.task_registry.lock().unwrap();
        reg.remove(task_id);
    }

    pub fn cancel_task(&self, task_id: &str) -> bool {
        let reg = self.task_registry.lock().unwrap();
        if let Some(flag) = reg.get(task_id) {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}
