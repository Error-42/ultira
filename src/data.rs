use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Data {
    pub config: Config,
    pub ratings: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub score_multiplier: f64,
    pub realloc: f64,
    pub default_rating: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            score_multiplier: 1.0,
            realloc: 0.01,
            default_rating: Some(100.0),
        }
    }
}
