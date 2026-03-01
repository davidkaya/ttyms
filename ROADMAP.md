# ttyms Roadmap

Feature roadmap for the terminal Microsoft Teams client. Most features below are achievable using existing [Microsoft Graph API](https://learn.microsoft.com/en-us/graph/api/resources/teams-api-overview) endpoints with delegated permissions. Items marked ❌ are not feasible with current API capabilities.

---

## ✅ Shipped

- 1:1 chat messaging (send/receive)
- Group chat messaging
- New chat creation with user search + autocomplete
- Device code flow + PKCE browser flow authentication
- Secure token storage (OS keyring + file fallback with zeroize)
- Auto-refresh (15s interval)
- Vim-style keyboard navigation
- **Unread indicators & badge counts** — per-chat unread count + total in header
- **Message reactions** — display reactions inline, add reactions via keyboard picker (👍❤️😂😮😢😡)
- **Rich text rendering** — bold, italic, code, links rendered with terminal formatting
- **Message read receipts** — chats marked as read when viewed
- **User presence / status** — see availability (🟢🔴⛔🟡⚫) for contacts and own status
- **Set your own presence** — change status via presence picker dialog
- **New message notification** — terminal bell on incoming messages
- **List joined teams** — browse all teams in tabbed Teams view
- **Channel browsing** — list channels within a team (standard + private)
- **Channel messages** — read and post messages in team channels
- **Tabbed UI** — switch between Chats and Teams views with 1/2 keys
- **Reply to messages** — quote-reply via `r` key when message selected (chat + channel)
- **Edit / delete messages** — edit own messages with `w`, soft-delete with `d`
- **Message pagination** — scroll up to load older messages via `@odata.nextLink`
- **Homebrew distribution** — `brew install davidkaya/tap/ttyms` with pre-built macOS binaries
- **Channel member list** — toggle member sidebar with `m` key, owners marked with 👑
- **Settings dialog** — configurable refresh interval via in-app settings
- **Delta-based message sync** — incremental message updates via Graph delta queries
- **Message search** — full-text search across chats via Microsoft Search API (`/search/query`)
- **Chat management** — rename group chats, view/add/remove members, leave chats via `g` key
- **Command palette** — `Ctrl+P` fuzzy-find across chats, channels, and actions
- **Share files in chat** — upload and share files via `f` key (up to 4MB, OneDrive-backed)

- **Mouse support** — click to select chats/teams/channels, scroll messages, focus panels

---

## 🔥 Phase 1 — Core Messaging Polish

~~Essential improvements to make daily use practical.~~ **SHIPPED**

### ~~Unread indicators & badge counts~~ ✅
### ~~Message read receipts~~ ✅
### ~~Message reactions (emoji)~~ ✅
### ~~Rich text rendering~~ ✅
### ~~Reply to specific messages~~ ✅
### ~~Delete / edit sent messages~~ ✅
### ~~Message pagination (infinite scroll)~~ ✅

---

## 🟡 Phase 2 — Presence & Notifications

~~Make the client feel alive and connected.~~ **SHIPPED**

### ~~User presence / status~~ ✅
### ~~Set your own presence~~ ✅
### ~~Desktop notifications~~ ✅ (terminal bell on new messages)

### ~~Typing indicators~~ ❌
~~Show "User is typing…" and broadcast your own typing state.~~
- **Not feasible** — Microsoft Graph API does not expose typing indicator endpoints for reading or broadcasting typing state. Would require SignalR/WebSocket which is not available for 3rd-party clients.

---

## 🟢 Phase 3 — Teams & Channels

~~Extend beyond 1:1/group chats into full Teams workspace support.~~ **SHIPPED**

### ~~List joined teams~~ ✅
### ~~Channel browsing~~ ✅
### ~~Channel messages~~ ✅ (read and send)
### ~~Channel member list~~ ✅

---

## 🔵 Phase 4 — File Sharing & Media

### ~~Share files in chat~~ ✅
~~Upload and share files within a chat conversation.~~
- ~~`PUT /me/drive/root:/Microsoft Teams Chat Files/{filename}:/content` (upload to OneDrive)~~
- ~~Send message with `attachment` referencing the uploaded file~~
- ~~Scope: `Files.ReadWrite`~~

### View shared files
List files shared in a chat and open them (launch in browser or download).
- `GET /me/chats/{id}/tabs` — pinned files
- Parse `attachment` objects from messages
- `GET /drives/{id}/items/{id}` — download URL

### Image previews
Render inline images in the terminal using unicode block characters or sixel protocol (for supported terminals).
- `GET /me/chats/{id}/messages/{id}/hostedContents/{id}` — fetch hosted image content

---

## 🟣 Phase 5 — Real-time & Advanced

### ~~WebSocket/SignalR real-time messages~~ → Delta-based sync ✅
~~Replace polling with real-time message delivery using Graph change notifications.~~
- True WebSocket/SignalR notifications require a public webhook URL (not feasible for terminal clients)
- Implemented **delta queries** (`/chats/{id}/messages/delta`) for incremental message sync
- Only new/changed messages are fetched on each poll cycle, dramatically reducing API calls
- Delta tokens stored per-chat for efficient incremental updates

### ~~Search messages~~ ✅
~~Full-text search across all chats and channels.~~
- ~~`GET /me/chats/{id}/messages?$search="query"` (limited)~~
- ~~`POST /search/query` — Microsoft Search API with `chatMessage` entity type~~
- ~~Scope: `Chat.Read`~~

### ~~Chat management~~ ✅
~~Rename group chats, add/remove members, leave a chat.~~
- ~~`PATCH /me/chats/{id}` — update topic~~
- ~~`POST /me/chats/{id}/members` — add member~~
- ~~`DELETE /me/chats/{id}/members/{id}` — remove member~~
- ~~`DELETE /me/chats/{id}/members/{myId}` — leave chat~~

### Create group chats
Create new group conversations (not just 1:1).
- `POST /chats` with `chatType: "group"` and multiple members
- Already partially implemented — extend `create_chat()` to accept multiple participants

### Pin / archive chats
Pin important chats to the top, archive inactive ones.
- `POST /me/chats/{id}/pinnedMessages` — pin a message
- `PATCH /me/chats/{id}` — hide/archive

### Contact / people list
Browse your frequent contacts and org directory.
- `GET /me/people` — ranked relevant contacts
- `GET /me/contacts` — address book
- Scope: `People.Read`

---

## 🧪 Phase 6 — Power User Features

### Multiple account support
Switch between different Microsoft 365 tenants/accounts.
- Store multiple token sets in keyring with tenant-scoped keys
- Config: `[[accounts]]` array in TOML

### Chat export
Export chat history to markdown, JSON, or plain text.
- Paginate through `GET /me/chats/{id}/messages` and serialize locally

### Keyboard macro / shortcuts customization
User-configurable keybindings via `config.toml`.
- No API — local config feature

### Theme customization
User-selectable color themes (dark, light, solarized, nord, etc.).
- No API — ratatui styling via config

### ~~Mouse support~~ ✅
~~Click to select chats, scroll messages, focus input.~~
- ~~No API — crossterm mouse event handling (already available in the dependency)~~

### ~~Command palette~~ ✅
~~`Ctrl+P` fuzzy-find across chats, channels, people, and actions.~~
- ~~Combine results from `/me/chats`, `/me/joinedTeams`, `/me/people`~~

### Markdown message composition
Write messages in markdown, convert to Teams-compatible HTML before sending.
- `POST /me/chats/{id}/messages` with `contentType: "html"` and converted body

### Adaptive Card rendering
Render incoming Adaptive Cards (approval requests, forms, polls) as structured terminal UI.
- Parse `attachment` objects with `contentType: "application/vnd.microsoft.card.adaptive"`

---

## 📦 Distribution

### ~~Homebrew (macOS)~~ ✅
Publish ttyms as a Homebrew formula for easy installation on macOS.
- Create a Homebrew tap repository (`homebrew-tap`)
- Add formula with `cargo install` or pre-built binaries from GitHub Releases
- Support `brew install ttyms` for one-command installation
- Auto-update formula on new GitHub Releases via CI

---

## Scope Requirements Summary

Scopes needed beyond what's currently configured:

| Phase | Additional Scopes |
|-------|-------------------|
| 1 | — (none, all within current scopes) |
| 2 | `Presence.Read`, `Presence.ReadWrite` (optional) |
| 3 | `Channel.ReadBasic.All`, `ChannelMessage.Read.All`, `ChannelMessage.Send`, `Team.ReadBasic.All` |
| 4 | `Files.ReadWrite` |
| 5 | `People.Read` |

---

## Priority Recommendation

For maximum impact with minimum effort:

1. **Unread indicators** — biggest UX win, zero new API scopes
2. **Message reactions** — makes the client feel complete
3. **Rich text rendering** — no API changes, pure client-side improvement
4. **User presence** — one new scope, huge quality-of-life improvement
5. **Teams & channels** — opens up the full Teams experience
