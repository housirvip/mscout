use super::value::ValueType;
use serde::{Deserialize, Serialize};

pub struct ScanResultSet {
    pub(crate) addresses: Vec<usize>,
    pub(crate) previous_values: Vec<u8>,
    pub(crate) history: Vec<(Vec<usize>, Vec<u8>)>,
    pub(crate) max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub address: String,
    pub value: String,
    pub previous_value: String,
}

impl ScanResultSet {
    pub fn new(_value_type: ValueType) -> Self {
        Self {
            addresses: Vec::new(),
            previous_values: Vec::new(),
            history: Vec::new(),
            max_history: 10,
        }
    }

    pub fn len(&self) -> usize {
        self.addresses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }

    pub fn push_history(&mut self) {
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history
            .push((self.addresses.clone(), self.previous_values.clone()));
    }

    pub fn pop_history(&mut self) -> bool {
        if let Some((addrs, vals)) = self.history.pop() {
            self.addresses = addrs;
            self.previous_values = vals;
            true
        } else {
            false
        }
    }
}
