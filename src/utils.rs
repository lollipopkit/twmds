use anyhow::{anyhow, Result};
use std::fs;

pub fn create_dir_ignore_exists(dir: &str) -> Result<()> {
    if let Err(err) = fs::create_dir(dir) {
        let err = err.to_string();
        if !err.contains("exists") {
            println!("🚫 {}", err);
            return Err(anyhow!(err));
        }
    }
    Ok(())
}

pub fn secs_to_human(secs: u64) -> String {
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        return format!("{}m", secs / 60);
    }
    format!("{}h", secs / 3600)
}
