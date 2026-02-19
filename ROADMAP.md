# ttyms Roadmap

Feature roadmap for the terminal Microsoft Teams client. All features below are achievable using existing [Microsoft Graph API](https://learn.microsoft.com/en-us/graph/api/resources/teams-api-overview) endpoints with delegated permissions.

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

---

## 🔥 Phase 1 — Core Messaging Polish

~~Essential improvements to make daily use practical.~~ **SHIPPED**

### ~~Unread indicators & badge counts~~ ✅
### ~~Message read receipts~~ ✅
### ~~Message reactions (emoji)~~ ✅
### ~~Rich text rendering~~ ✅

### Reply to specific messages
Quote-reply to a message in a chat thread.
- `POST /me/chats/{id}/messages` — with `replyToId` in the request body

### Delete / edit sent messages
Edit or soft-delete your own messages.
- `PATCH /me/chats/{id}/messages/{id}` — update body content
- `DELETE /me/chats/{id}/messages/{id}` (soft delete)

### Message pagination (infinite scroll)
Load older messages when scrolling to the top using `@odata.nextLink`.
- `GET /me/chats/{id}/messages?$top=50&$skiptoken=...`

---

## 🟡 Phase 2 — Presence & Notifications

~~Make the client feel alive and connected.~~ **SHIPPED**

### ~~User presence / status~~ ✅
### ~~Set your own presence~~ ✅
### ~~Desktop notifications~~ ✅ (terminal bell on new messages)

### Typing indicators
Show "User is typing…" and broadcast your own typing state.
- `POST /me/chats/{id}/sendActivityNotification` (limited)
- Realistically requires SignalR/WebSocket subscription (see Phase 5)

---

## 🟢 Phase 3 — Teams & Channels

~~Extend beyond 1:1/group chats into full Teams workspace support.~~ **SHIPPED**

### ~~List joined teams~~ ✅
### ~~Channel browsing~~ ✅
### ~~Channel messages~~ ✅ (read and send)

### Channel member list
View members of a channel.
- `GET /teams/{id}/channels/{id}/members`

---

## 🔵 Phase 4 — File Sharing & Media

### Share files in chat
Upload and share files within a chat conversation.
- `PUT /me/chats/{id}/files/content` (upload to OneDrive for Business)
- Send message with `attachment` referencing the uploaded file
- Scope: `Files.ReadWrite`

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

### WebSocket/SignalR real-time messages
Replace polling with real-time message delivery using Graph change notifications.
- `POST /subscriptions` — subscribe to `/me/chats/getAllMessages`
- Requires a notification URL or uses websocket transport (beta)
- Dramatically reduces latency and API calls

### Search messages
Full-text search across all chats and channels.
- `GET /me/chats/{id}/messages?$search="query"` (limited)
- `POST /search/query` — Microsoft Search API with `chatMessage` entity type
- Scope: `Chat.Read`

### Chat management
Rename group chats, add/remove members, leave a chat.
- `PATCH /me/chats/{id}` — update topic
- `POST /me/chats/{id}/members` — add member
- `DELETE /me/chats/{id}/members/{id}` — remove member
- `DELETE /me/chats/{id}/members/{myId}` — leave chat

### Create group chats
Create new group conversations (not just 1:1).
- `POST /chats` with `chatType: "group"` and multiple members
- Already partially implemented — extend `create_chat()` to accept multiple participants

### Pin / archive chats
Pin important chats to the top, archive inactive ones.
- `POST /me/chats/{id}/pinnedMessages` — pin a message
- `PATCH /me/chats/{id}` — hide/archive

### Contact / people list
Browse your frequent contacts and org directory.C:\Users\dvidkaya\AppData\Roaming\ttyms\config.toml.C:\Users\dvidkaya\AppData\Roaming\ttyms\config.toml.
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

### Mouse support
Click to select chats, scroll messages, focus input.
- No API — crossterm mouse event handling (already available in the dependency)

### Command palette
`Ctrl+P` fuzzy-find across chats, channels, people, and actions.
- Combine results from `/me/chats`, `/me/joinedTeams`, `/me/people`

### Markdown message composition
Write messages in markdown, convert to Teams-compatible HTML before sending.
- `POST /me/chats/{id}/messages` with `contentType: "html"` and converted body

### Adaptive Card rendering
Render incoming Adaptive Cards (approval requests, forms, polls) as structured terminal UI.
- Parse `attachment` objects with `contentType: "application/vnd.microsoft.card.adaptive"`

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
