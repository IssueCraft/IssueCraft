use facet::Facet;

const DEFAULT_DB_PATH: &str = ".ic.db";

#[derive(Debug, Facet)]
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
