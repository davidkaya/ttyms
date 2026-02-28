# Changelog

All notable changes to ttyms will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Delta-based message sync** — uses Microsoft Graph delta queries (`/messages/delta`) for incremental message updates instead of full re-fetching, reducing API calls and latency
- **Message search** — full-text search across all chats via `/` key, powered by Microsoft Search API (`POST /search/query` with `chatMessage` entity type); shows sender, timestamp, and summary snippet; Enter on a result navigates to the chat
- **Chat management** — manage chats via `g` key: tabbed dialog with member list (view/remove), rename group chats, and add members with user search; leave chat via `l` key
- **Command palette** — `Ctrl+P` opens a fuzzy-find palette across all chats, team channels, and quick actions (New Chat, Search, Set Status, Settings, Quit); type to filter, arrow keys to navigate, Enter to execute

## [0.1.4] - 2026-02-27

### Added

- **Settings dialog** — configurable refresh interval via in-app settings UI
- Dependabot enabled for automated Cargo dependency updates

### Changed

- Updated all dependencies to latest versions: ratatui 0.30, crossterm 0.29, reqwest 0.13, keyring 3, rand 0.10, dirs 6, base64 0.22, toml 1.0
- Migrated reqwest TLS feature from `rustls-tls` to `rustls` (renamed in 0.13)
- Migrated deprecated ratatui APIs (`frame.size()` → `frame.area()`, `frame.set_cursor()` → `frame.set_cursor_position()`)
- Auto-refresh moved to background with parallelized presence loading for snappier UI

### Fixed

- API errors now shown as modal dialogs instead of truncated status bar text
- Channel permission errors (403) displayed as modal dialog with clear guidance
- Graceful handling of missing `ChannelMessage.Read.All` permission

## [0.1.3] - 2026-02-27

### Added

- **Channel member list** — toggle member sidebar with `m` key in channel views, owners marked with 👑
- **Security hardening** — OData filter injection prevention, file token storage with restricted permissions (0600), `.gitignore` for sensitive files
- **Keyring fallback warning** — users are notified when token storage falls back to file instead of OS credential manager

### Fixed

- OData injection vulnerability in user search (single-quote escaping)

## [0.1.2] - 2026-02-27

### Added

- **Reply to messages** — quote-reply to any message with `r` key (chats + channels)
- **Edit & delete messages** — edit own messages with `w`, soft-delete with `d`
- **Message pagination** — scroll up to load older messages automatically via `@odata.nextLink`
- **Homebrew distribution** — `brew install davidkaya/tap/ttyms` with pre-built binaries for macOS (x86_64 + aarch64) and Linux
- **GitHub Releases** — binary archives published for macOS and Linux (4 targets)

### Changed

- Switched from OpenSSL (native-tls) to rustls for cross-platform compatibility
- CI uses GitHub App tokens instead of PAT for Homebrew tap publishing

### Fixed

- Cross-compilation failures on aarch64-linux (OpenSSL headers)
- macOS CI runner deprecation (macos-13 → macos-14)
- Graceful handling of duplicate crate versions on crates.io

## [0.1.1] - 2026-02-26

### Added

- **AUR packages** — `ttyms` (release) and `ttyms-git` (VCS) packages for Arch Linux
- **crates.io publishing** — automated publish on release
- Graceful fallback when OS keyring is unavailable (file-based token storage)

## [0.1.0] - 2026-02-26

### Added

- **1:1 and group chat messaging** — browse chats, read and send messages
- **Teams & Channels** — browse joined teams, navigate channels, read and post channel messages
- **New chat creation** — user search with autocomplete
- **Authentication** — device code flow + PKCE browser flow with secure token storage
- **Message reactions** — view and add reactions (👍❤️😂😮😢😡)
- **Rich text rendering** — bold, italic, code, and links with terminal formatting
- **User presence** — see and set online status (Available, Busy, DND, Away, Offline)
- **Unread indicators** — per-chat unread counts and total badge in header
- **Auto-refresh** — messages update every 15 seconds with terminal bell on new messages
- **Vim-style navigation** — `j`/`k` and arrow keys throughout
- **Tabbed UI** — switch between Chats and Teams views with `1`/`2` keys
- **Background preloading** — cached teams/channels for instant navigation
