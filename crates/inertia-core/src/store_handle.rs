use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::runtime::RuntimeFlavor;

use crate::error::{CoreError, CoreResult};
use crate::storage::Store;

/// Thread-safe async access to SQLite (rusqlite is not `Sync`).
#[derive(Clone)]
pub struct StoreHandle(Arc<Mutex<Store>>);

impl StoreHandle {
    pub fn open(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        Ok(Self(Arc::new(Mutex::new(Store::open(data_dir)?))))
    }

    pub async fn with<T, F>(&self, f: F) -> CoreResult<T>
    where
        F: FnOnce(&Store) -> CoreResult<T>,
    {
        let inner = Arc::clone(&self.0);
        if should_block_in_place() {
            tokio::task::block_in_place(|| {
                let store = lock_store(&inner)?;
                f(&store)
            })
        } else {
            let store = lock_store(&inner)?;
            f(&store)
        }
    }

    pub async fn with_mut<T, F>(&self, f: F) -> CoreResult<T>
    where
        F: FnOnce(&mut Store) -> CoreResult<T>,
    {
        let inner = Arc::clone(&self.0);
        if should_block_in_place() {
            tokio::task::block_in_place(|| {
                let mut store = lock_store(&inner)?;
                f(&mut store)
            })
        } else {
            let mut store = lock_store(&inner)?;
            f(&mut store)
        }
    }
}

fn should_block_in_place() -> bool {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.runtime_flavor() == RuntimeFlavor::MultiThread,
        Err(_) => false,
    }
}

fn lock_store(inner: &Arc<Mutex<Store>>) -> CoreResult<MutexGuard<'_, Store>> {
    inner
        .lock()
        .map_err(|_| CoreError::P2p("store lock poisoned".into()))
}
