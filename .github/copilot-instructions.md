# Copilot Instructions for ttyms

## Project Overview
ttyms is a terminal-based Microsoft Teams client written in Rust using ratatui for the TUI.
It uses the Microsoft Graph API for messaging and OAuth2 for authentication.

## Code Quality Rules

### General
- Follow idiomatic Rust patterns and conventions
- Use `anyhow::Result` for error handling in application code
- Prefer descriptive variable and function names over comments
- Keep functions focused and under 50 lines where possible
- No `unwrap()` in production code — use `?`, `.unwrap_or()`, or proper error handling
- All public functions must have a clear purpose evident from their name

### Security
- **Never log or print tokens, secrets, or credentials**
- All sensitive data (tokens, keys) must implement `Zeroize` or be manually zeroized on drop
- Tokens must be stored in the OS credential manager (keyring crate), never in plain text files
- Use HTTPS for all external API calls
- Request minimal OAuth scopes necessary for functionality
- Sanitize all user input before sending to APIs

### Architecture
- Keep modules focused: `auth.rs` for auth, `client.rs` for API calls, `models.rs` for data, etc.
- Separate UI rendering (`ui.rs`) from state management (`app.rs`)
- The `App` struct is the single source of truth for application state
- Graph API interactions go through `GraphClient` only

### Testing
- **All new features must include tests**
- Tests live in `tests/` as integration tests
- Test file naming: `<module>_tests.rs`
- Organize tests into `mod` blocks by functionality
- Test both happy paths and edge cases (empty inputs, None values, boundary conditions)
- Use helper functions to create test fixtures (e.g., `make_chat()`, `make_message()`)
- No network calls in tests — mock or test only pure logic
- Run `cargo test` before committing — all tests must pass

### Dependencies
- Pin major versions in Cargo.toml
- Prefer well-maintained crates with minimal dependency trees
- Security-sensitive crates (keyring, zeroize, sha2) must be from trusted sources

### UI Guidelines
- Active panel indicated by cyan border color
- Keyboard shortcuts shown in the status bar
- Vim-style navigation (j/k) alongside arrow keys
- All dialogs must be dismissible with Esc
- Status messages should be concise and truncated to fit terminal width

### Git Conventions
- **Always use Conventional Commits** (https://www.conventionalcommits.org/)
  - Format: `<type>(<scope>): <description>`
  - Types: `feat`, `fix`, `docs`, `test`, `ci`, `refactor`, `chore`, `perf`, `style`
  - Scopes: `auth`, `ui`, `client`, `models`, `app`, `config`, or omit for broad changes
  - Examples: `feat(auth): add PKCE browser flow`, `fix(ui): truncate status bar text`
- Include `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailer
- Keep commits focused on a single logical change

### Roadmap Maintenance
- **Always update `ROADMAP.md` when completing features** — mark items with ~~strikethrough~~ and ✅
- When a roadmap item is implemented, also add it to the **✅ Shipped** section at the top
- When all items in a phase are complete, mark the phase header as **SHIPPED**
- When adding new features not on the roadmap, add them to the appropriate phase or create a new section
