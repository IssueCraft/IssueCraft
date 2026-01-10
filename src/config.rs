use std::path::{Path, PathBuf};

use facet::Facet;

const DEFAULT_DB_PATH: &str = ".ic.db";

#[derive(Debug, Facet)]
pub struct Config {
    pub db_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: directories::BaseDirs::new()
                .map(|bd| bd.data_local_dir().to_path_buf())
                .unwrap_or_else(|| "~/.local/share".into())
                .join("issuecraft")
                .join(DEFAULT_DB_PATH),
        }
    }
}
