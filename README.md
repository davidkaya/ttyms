# Terms — Terminal Microsoft Teams Client

A secure, fast terminal UI client for Microsoft Teams messaging, built in Rust with [ratatui](https://ratatui.rs/).

![Rust](https://img.shields.io/badge/Rust-1.83+-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **1:1 and group chat messaging** — browse all your Teams chats, read messages, and reply
- **Beautiful TUI** — clean terminal interface with panels, color-coded messages, and keyboard navigation
- **Secure by design** — tokens stored in OS credential manager (Windows Credential Manager / macOS Keychain / Linux Secret Service), sensitive data zeroized in memory
- **Auto-refresh** — messages update automatically every 15 seconds
- **Vim-style navigation** — use `j`/`k` or arrow keys to navigate

## Screenshots

```
┌─────────────────────────────────────────────────────────────────┐
│ ◆ TERMS │ Microsoft Teams                        John Doe      │
├───────────────────┬─────────────────────────────────────────────┤
│ Chats             │ Project Discussion                          │
│                   │─────────────────────────────────────────────│
│ ▸ Project Disc…   │ Alice Smith                       10:30    │
│   Hey team, the…  │   Hey team, the deployment went well!      │
│                   │                                             │
│   Alice Smith     │ Bob Johnson                       10:32    │
│   Can you review… │   Great news! Any issues?                  │
│                   │                                             │
│   Dev Team        │ You                               10:33    │
│   Meeting at 3pm  │   Nope, all smooth!                        │
│                   │─────────────────────────────────────────────│
│                   │ Message                                     │
│                   │ > Great work everyone!                      │
├───────────────────┴─────────────────────────────────────────────┤
│ Tab Switch │ Enter Send │ ↑↓ Navigate │ r Refresh │ q Quit     │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

- **Rust 1.83+** — install via [rustup](https://rustup.rs/)
- **Microsoft 365 account** with Teams access
- **Azure AD app registration** (see setup below)

## Setup

### 1. Register an Azure AD Application

1. Go to [Azure Portal](https://portal.azure.com) → **Microsoft Entra ID** → **App registrations**
2. Click **New registration**
3. Set:
   - **Name**: `Terms` (or any name)
   - **Supported account types**: *Accounts in any organizational directory* (for multi-tenant)
   - **Redirect URI**: leave blank
4. Click **Register**
5. Go to **Authentication**:
   - Enable **Allow public client flows** → Save
   - Click **Add a platform** → **Mobile and desktop applications**
   - Under custom redirect URIs, add: `http://localhost` → Save
6. Go to **API permissions** → **Add a permission** → **Microsoft Graph** → **Delegated permissions**:
   - `User.Read`
   - `Chat.Read`
   - `ChatMessage.Read`
   - `ChatMessage.Send`
7. Copy the **Application (client) ID**

### 2. Configure Terms

Run the app once to generate the config file:

```sh
cargo run
```

Edit the config file (location shown in the output):
- **Windows**: `%APPDATA%\terms\config.toml`
- **macOS**: `~/Library/Application Support/terms/config.toml`
- **Linux**: `~/.config/terms/config.toml`

Set your `client_id`:

```toml
client_id = "your-application-client-id-here"
tenant_id = "common"
```

### 3. Run

```sh
cargo run
```

On first run, you'll be prompted to sign in using the device code flow.

### Authentication Options

**Device Code Flow (default)** — displays a code, you sign in via browser:
```sh
cargo run
```

**PKCE Browser Flow** — browser opens automatically, redirects to localhost:
```sh
cargo run -- --pkce
```

## Usage

### Keyboard Shortcuts

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Switch between panels (Chats → Messages → Input) |
| `↑`/`↓` or `j`/`k` | Navigate chats / scroll messages |
| `Enter` | Send message / jump to input |
| `r` | Refresh chats and messages |
| `Esc` | Back to chat list |
| `q` | Quit |
| `Ctrl+C` | Force quit |

### CLI Options

```sh
terms --help     # Show help
terms --pkce     # Use PKCE browser flow instead of device code
terms --logout   # Clear stored credentials securely
```

## Security

| Concern | Mitigation |
|---|---|
| Token storage | OS credential manager via [`keyring`](https://crates.io/crates/keyring) crate |
| Memory safety | Tokens zeroized on drop via [`zeroize`](https://crates.io/crates/zeroize) crate |
| Auth flow | OAuth2 Device Code Flow (public client, no client secret stored) |
| Transport | All API calls over HTTPS to Microsoft Graph |
| Scopes | Minimal permissions: only User.Read, Chat.Read, ChatMessage.Read, ChatMessage.Send |
| Logout | `--logout` securely removes credentials from OS store |

## Building

```sh
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

## License

MIT
