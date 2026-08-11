use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::platform::ProcessMemory;
use crate::scanner::ScanValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenEntry {
    pub address: usize,
    pub value: ScanValue,
    pub enabled: bool,
    pub label: String,
}

pub struct FreezeManager {
    entries: Arc<Mutex<Vec<FrozenEntry>>>,
    interval_ms: Arc<Mutex<u64>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl FreezeManager {
    /// Start the freeze manager with a background thread that periodically writes frozen values.
    pub fn start(process: Arc<dyn ProcessMemory + Send + Sync>) -> Self {
        let entries: Arc<Mutex<Vec<FrozenEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let interval_ms = Arc::new(Mutex::new(100u64));
        let running = Arc::new(AtomicBool::new(true));

        let entries_clone = Arc::clone(&entries);
        let interval_clone = Arc::clone(&interval_ms);
        let running_clone = Arc::clone(&running);

        let thread_handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                let interval = {
                    let ms = interval_clone.lock().unwrap();
                    Duration::from_millis(*ms)
                };

                {
                    let entries_guard = entries_clone.lock().unwrap();
                    for entry in entries_guard.iter() {
                        if entry.enabled {
                            let data = entry.value.to_bytes();
                            let _ = process.write(entry.address, &data);
                        }
                    }
                }

                thread::sleep(interval);
            }
        });

        Self {
            entries,
            interval_ms,
            running,
            thread_handle: Some(thread_handle),
        }
    }

    /// Add a new frozen entry.
    pub fn add(&self, address: usize, value: ScanValue, label: String) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(FrozenEntry {
            address,
            value,
            enabled: true,
            label,
        });
    }

    /// Remove a frozen entry by address.
    pub fn remove(&self, address: usize) -> bool {
        let mut entries = self.entries.lock().unwrap();
        let len_before = entries.len();
        entries.retain(|e| e.address != address);
        entries.len() < len_before
    }

    /// Enable or disable a frozen entry by address.
    pub fn set_enabled(&self, address: usize, enabled: bool) -> bool {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.iter_mut().find(|e| e.address == address) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Set the freeze write interval in milliseconds.
    pub fn set_interval(&self, ms: u64) {
        let mut interval = self.interval_ms.lock().unwrap();
        *interval = ms;
    }

    /// Get a snapshot of all frozen entries.
    pub fn list(&self) -> Vec<FrozenEntry> {
        let entries = self.entries.lock().unwrap();
        entries.clone()
    }

    /// Stop the freeze thread.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for FreezeManager {
    fn drop(&mut self) {
        self.stop();
    }
}
