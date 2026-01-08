use serde::{Deserialize, Serialize};

const DEFAULT_CONNECTION_STRING: &str = ".ic.db";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub connection_string: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connection_string: DEFAULT_CONNECTION_STRING.to_string(),
        }
    }
}
