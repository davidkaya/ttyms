use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub client_id: String,
    pub tenant_id: String,
}

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("terms");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn load_config() -> Result<Config> {
    let path = config_dir()?.join("config.toml");
    if !path.exists() {
        let template = r#"# Terms Configuration
# Tokens are stored securely in your OS credential manager.
# This file contains non-sensitive settings only.

# Azure AD Application (client) ID (required)
client_id = ""

# Azure AD Tenant ID ("common" for multi-tenant, or your specific tenant ID)
tenant_id = "common"
"#;
        std::fs::write(&path, template)?;
        anyhow::bail!(
            "Config created at {}. Please set your client_id.",
            path.display()
        );
    }
    let content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content).context("Invalid config file format")?;
    if config.client_id.is_empty() {
        anyhow::bail!("Please set client_id in {}", path.display());
    }
    Ok(config)
}

pub fn print_setup_guide() {
    eprintln!("Setup Guide:");
    eprintln!("  1. Go to https://portal.azure.com → Microsoft Entra ID → App registrations");
    eprintln!("  2. Click 'New registration'");
    eprintln!("  3. Name: 'Terms' (or any name you prefer)");
    eprintln!("  4. Supported account types: 'Accounts in any organizational directory'");
    eprintln!("  5. Redirect URI: leave blank");
    eprintln!("  6. After creation, go to 'Authentication':");
    eprintln!("     - Enable 'Allow public client flows' → Save");
    eprintln!("     - Add platform → 'Mobile and desktop applications' → add: http://localhost");
    eprintln!("  7. Go to 'API permissions' → Add permissions:");
    eprintln!("     - Microsoft Graph → Delegated: User.Read, Chat.Read, ChatMessage.Read, ChatMessage.Send");
    eprintln!("  8. Copy the 'Application (client) ID' to your config file");
}
