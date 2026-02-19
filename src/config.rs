use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default Azure AD client ID for ttyms
pub const DEFAULT_CLIENT_ID: &str = "ac138a64-055b-4915-b670-31200c6235e6";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub client_id: String,
    pub tenant_id: String,
}

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("ttyms");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn load_config() -> Result<Config> {
    let path = config_dir()?.join("config.toml");
    if !path.exists() {
        let template = format!(
            r#"# ttyms Configuration
# Tokens are stored securely in your OS credential manager.
# This file contains non-sensitive settings only.

# Azure AD Application (client) ID
# Leave empty to use the built-in default: {}
client_id = ""

# Azure AD Tenant ID ("common" for multi-tenant, or your specific tenant ID)
tenant_id = "common"
"#,
            DEFAULT_CLIENT_ID
        );
        std::fs::write(&path, template)?;
    }
    let content = std::fs::read_to_string(&path)?;
    let mut config: Config = toml::from_str(&content).context("Invalid config file format")?;
    if config.client_id.is_empty() {
        config.client_id = DEFAULT_CLIENT_ID.to_string();
    }
    Ok(config)
}

pub fn print_setup_guide() {
    eprintln!("Setup Guide:");
    eprintln!("  1. Go to https://portal.azure.com → Microsoft Entra ID → App registrations");
    eprintln!("  2. Click 'New registration'");
    eprintln!("  3. Name: 'ttyms' (or any name you prefer)");
    eprintln!("  4. Supported account types: 'Accounts in any organizational directory'");
    eprintln!("  5. Redirect URI: leave blank");
    eprintln!("  6. After creation, go to 'Authentication':");
    eprintln!("     - Enable 'Allow public client flows' → Save");
    eprintln!("     - Add platform → 'Mobile and desktop applications' → add: http://localhost");
    eprintln!("  7. Go to 'API permissions' → Add permissions:");
    eprintln!("     - Microsoft Graph → Delegated:");
    eprintln!("       User.Read, User.ReadBasic.All, Chat.ReadWrite,");
    eprintln!("       ChatMessage.Read, ChatMessage.Send,");
    eprintln!("       Presence.Read, Presence.ReadWrite,");
    eprintln!("       Team.ReadBasic.All, Channel.ReadBasic.All,");
    eprintln!("       ChannelMessage.Read.All, ChannelMessage.Send");
    eprintln!("  8. Copy the 'Application (client) ID' to your config file");
}
