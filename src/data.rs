use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Data {
    pub config: Config,
    pub ratings: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub k: f64,
    pub spread: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self { k: 1.0, spread: 100.0 }
    }
}