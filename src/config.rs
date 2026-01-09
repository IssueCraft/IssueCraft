use serde::{Deserialize, Serialize};

const DEFAULT_DB_PATH: &str = ".ic.db";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub db_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: DEFAULT_DB_PATH.to_string(),
        }
    }
}
