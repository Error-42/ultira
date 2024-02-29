use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Data {
    config: Config,
    ratings: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    k: f64,
}