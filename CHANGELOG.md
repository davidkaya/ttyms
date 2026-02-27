# Changelog

All notable changes to ttyms will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
