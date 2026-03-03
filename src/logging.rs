use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init_logging() -> Result<PathBuf> {
    let path = log_file_path()?;
    append_entry("INFO", "logger.initialized")?;
    Ok(path)
}

pub fn log_event(event: &str) -> Result<()> {
    validate_event_label(event)?;
    append_entry("INFO", event)
}

pub fn log_failure(operation: &str) -> Result<()> {
    validate_event_label(operation)?;
    append_entry("ERROR", &format!("failure.{}", operation))
}

pub fn log_file_path() -> Result<PathBuf> {
    if let Some(path) = LOG_FILE_PATH.get() {
        return Ok(path.clone());
    }

    let dir = standard_log_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create log directory: {}", dir.display()))?;
    let path = dir.join("ttyms.log");
    let _ = LOG_FILE_PATH.set(path.clone());
    Ok(path)
}

pub fn is_safe_event_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 80
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn validate_event_label(label: &str) -> Result<()> {
    if is_safe_event_label(label) {
        Ok(())
    } else {
        anyhow::bail!("Unsafe log label: use only ASCII letters, numbers, '.', '_' or '-'")
    }
}

fn append_entry(level: &str, event: &str) -> Result<()> {
    let path = log_file_path()?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open log file: {}", path.display()))?;
    use std::io::Write;
    writeln!(file, "{} [{}] {}", Utc::now().to_rfc3339(), level, event)
        .with_context(|| format!("Failed to write log file: {}", path.display()))?;
    Ok(())
}

fn standard_log_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return dirs::data_local_dir()
            .context("Could not determine LocalAppData directory")
            .map(|d| d.join("ttyms").join("logs"));
    }

    #[cfg(target_os = "macos")]
    {
        return dirs::home_dir()
            .context("Could not determine home directory")
            .map(|h| h.join("Library").join("Logs").join("ttyms"));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(state_dir) = dirs::state_dir() {
            return Ok(state_dir.join("ttyms").join("logs"));
        }

        return dirs::home_dir()
            .context("Could not determine home directory")
            .map(|h| h.join(".local").join("state").join("ttyms").join("logs"));
    }
}
