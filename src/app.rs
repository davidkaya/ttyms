use std::collections::{HashMap, HashSet};

use crate::models::{Channel, ChannelMember, Chat, Message, Team, User};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UserSuggestion {
    pub display_name: String,
    pub email: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    ChatList,
    Messages,
    Input,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Chats,
    Teams,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TeamsPanel {
    TeamList,
    ChannelList,
    ChannelMessages,
    ChannelInput,
}

#[derive(Debug, Clone)]
pub enum AppScreen {
    Loading { message: String },
    Main,
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogMode {
    None,
    NewChat,
    ReactionPicker,
    PresencePicker,
    Error(ErrorInfo),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorInfo {
    pub title: String,
    pub message: String,
    pub details: String,
}

pub struct App {
    pub screen: AppScreen,
    pub active_panel: Panel,
    pub view_mode: ViewMode,

    // Chats
    pub chats: Vec<Chat>,
    pub selected_chat: usize,
    pub messages: Vec<Message>,
    pub input: String,
    pub input_cursor: usize,

    // User
    pub current_user: Option<User>,
    pub status_message: String,

    // Scrolling
    pub scroll_offset: usize,

    // Refresh
    pub last_refresh: std::time::Instant,
    pub refresh_interval: std::time::Duration,

    // New chat dialog
    pub new_chat_mode: bool,
    pub new_chat_input: String,
    pub new_chat_cursor: usize,
    pub suggestions: Vec<UserSuggestion>,
    pub selected_suggestion: usize,
    pub last_search_query: String,

    // Dialog mode
    pub dialog: DialogMode,

    // Message selection (for reactions)
    pub selected_message: Option<usize>,
    pub selected_channel_message: Option<usize>,
    pub selected_reaction: usize,

    // Presence
    pub my_presence: String,
    pub presence_map: HashMap<String, String>,
    pub selected_presence: usize,

    // Teams & Channels
    pub teams: Vec<Team>,
    pub selected_team: usize,
    pub channels: Vec<Channel>,
    pub selected_channel: usize,
    pub channel_messages: Vec<Message>,
    pub channel_input: String,
    pub channel_input_cursor: usize,
    pub channel_scroll_offset: usize,
    pub teams_panel: TeamsPanel,

    // Caches for instant navigation
    pub channels_cache: HashMap<String, Vec<Channel>>,
    pub channel_message_cache: HashMap<String, Vec<Message>>,

    // Channel members
    pub channel_members: Vec<ChannelMember>,
    pub show_members: bool,

    // Permission state
    pub channel_permission_denied: bool,

    // Unread tracking
    pub total_unread: i32,
    pub known_message_ids: HashSet<String>,

    // Reply state
    pub reply_to_message_id: Option<String>,
    pub reply_to_preview: String,

    // Edit state
    pub editing_message_id: Option<String>,

    // Pagination
    pub messages_next_link: Option<String>,
    pub channel_messages_next_link: Option<String>,
    pub loading_more_messages: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Loading {
                message: "Starting...".to_string(),
            },
            active_panel: Panel::ChatList,
            view_mode: ViewMode::Chats,
            chats: Vec::new(),
            selected_chat: 0,
            messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            current_user: None,
            status_message: String::new(),
            scroll_offset: 0,
            last_refresh: std::time::Instant::now(),
            refresh_interval: std::time::Duration::from_secs(15),
            new_chat_mode: false,
            new_chat_input: String::new(),
            new_chat_cursor: 0,
            suggestions: Vec::new(),
            selected_suggestion: 0,
            last_search_query: String::new(),
            dialog: DialogMode::None,
            selected_message: None,
            selected_channel_message: None,
            selected_reaction: 0,
            my_presence: "PresenceUnknown".to_string(),
            presence_map: HashMap::new(),
            selected_presence: 0,
            teams: Vec::new(),
            selected_team: 0,
            channels: Vec::new(),
            selected_channel: 0,
            channel_messages: Vec::new(),
            channel_input: String::new(),
            channel_input_cursor: 0,
            channel_scroll_offset: 0,
            teams_panel: TeamsPanel::TeamList,
            channels_cache: HashMap::new(),
            channel_message_cache: HashMap::new(),
            channel_members: Vec::new(),
            show_members: false,
            channel_permission_denied: false,
            total_unread: 0,
            known_message_ids: HashSet::new(),
            reply_to_message_id: None,
            reply_to_preview: String::new(),
            editing_message_id: None,
            messages_next_link: None,
            channel_messages_next_link: None,
            loading_more_messages: false,
        }
    }

    pub fn current_user_id(&self) -> &str {
        self.current_user
            .as_ref()
            .map(|u| u.id.as_str())
            .unwrap_or("")
    }

    pub fn selected_chat_id(&self) -> Option<&str> {
        self.chats.get(self.selected_chat).map(|c| c.id.as_str())
    }

    pub fn selected_chat_name(&self) -> String {
        self.chats
            .get(self.selected_chat)
            .map(|c| c.display_name(self.current_user_id()))
            .unwrap_or_else(|| "No chat selected".to_string())
    }

    pub fn select_next_chat(&mut self) {
        if !self.chats.is_empty() {
            self.selected_chat = (self.selected_chat + 1).min(self.chats.len() - 1);
        }
    }

    pub fn select_prev_chat(&mut self) {
        self.selected_chat = self.selected_chat.saturating_sub(1);
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::ChatList => Panel::Messages,
            Panel::Messages => Panel::Input,
            Panel::Input => Panel::ChatList,
        };
    }

    pub fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::ChatList => Panel::Input,
            Panel::Messages => Panel::ChatList,
            Panel::Input => Panel::Messages,
        };
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.input_cursor > 0 {
            let prev_len = self.input[..self.input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor -= prev_len;
            self.input.remove(self.input_cursor);
        }
    }

    pub fn take_input(&mut self) -> String {
        let input = self.input.clone();
        self.input.clear();
        self.input_cursor = 0;
        input
    }

    pub fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            let prev_len = self.input[..self.input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor -= prev_len;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.input_cursor < self.input.len() {
            let next_len = self.input[self.input_cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.input_cursor += next_len;
        }
    }

    pub fn scroll_messages_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    pub fn scroll_messages_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    pub fn mark_refreshed(&mut self) {
        self.last_refresh = std::time::Instant::now();
    }

    // ---- New chat mode ----

    pub fn enter_new_chat_mode(&mut self) {
        self.new_chat_mode = true;
        self.dialog = DialogMode::NewChat;
        self.new_chat_input.clear();
        self.new_chat_cursor = 0;
    }

    pub fn exit_new_chat_mode(&mut self) {
        self.new_chat_mode = false;
        self.dialog = DialogMode::None;
        self.new_chat_input.clear();
        self.new_chat_cursor = 0;
        self.suggestions.clear();
        self.selected_suggestion = 0;
        self.last_search_query.clear();
    }

    pub fn new_chat_insert_char(&mut self, c: char) {
        self.new_chat_input.insert(self.new_chat_cursor, c);
        self.new_chat_cursor += c.len_utf8();
    }

    pub fn new_chat_delete_char(&mut self) {
        if self.new_chat_cursor > 0 {
            let prev_len = self.new_chat_input[..self.new_chat_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.new_chat_cursor -= prev_len;
            self.new_chat_input.remove(self.new_chat_cursor);
        }
    }

    pub fn take_new_chat_input(&mut self) -> String {
        let input = self.new_chat_input.clone();
        self.exit_new_chat_mode();
        input
    }

    pub fn select_suggestion(&mut self) -> Option<String> {
        if let Some(s) = self.suggestions.get(self.selected_suggestion) {
            let email = s.email.clone();
            self.exit_new_chat_mode();
            Some(email)
        } else {
            None
        }
    }

    pub fn suggestion_up(&mut self) {
        self.selected_suggestion = self.selected_suggestion.saturating_sub(1);
    }

    pub fn suggestion_down(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_suggestion =
                (self.selected_suggestion + 1).min(self.suggestions.len() - 1);
        }
    }

    /// Returns true if search should be triggered (input changed and >= 2 chars)
    pub fn should_search(&self) -> bool {
        self.new_chat_input.len() >= 2 && self.new_chat_input != self.last_search_query
    }

    // ---- Message selection (for reactions) ----

    pub fn select_message_up(&mut self) {
        let user_msgs: Vec<usize> = self
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_user_message())
            .map(|(i, _)| i)
            .collect();

        if user_msgs.is_empty() {
            return;
        }

        match self.selected_message {
            None => {
                self.selected_message = user_msgs.last().copied();
            }
            Some(cur) => {
                if let Some(pos) = user_msgs.iter().position(|&i| i == cur) {
                    if pos > 0 {
                        self.selected_message = Some(user_msgs[pos - 1]);
                    }
                }
            }
        }
    }

    pub fn select_message_down(&mut self) {
        let user_msgs: Vec<usize> = self
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_user_message())
            .map(|(i, _)| i)
            .collect();

        if user_msgs.is_empty() {
            return;
        }

        match self.selected_message {
            None => return,
            Some(cur) => {
                if let Some(pos) = user_msgs.iter().position(|&i| i == cur) {
                    if pos + 1 < user_msgs.len() {
                        self.selected_message = Some(user_msgs[pos + 1]);
                    } else {
                        self.selected_message = None;
                    }
                }
            }
        }
    }

    pub fn selected_message_id(&self) -> Option<&str> {
        self.selected_message
            .and_then(|idx| self.messages.get(idx))
            .map(|m| m.id.as_str())
    }

    // ---- Channel message selection (for reactions in teams view) ----

    pub fn select_channel_message_up(&mut self) {
        let msgs: Vec<usize> = self
            .channel_messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_user_message())
            .map(|(i, _)| i)
            .collect();
        if msgs.is_empty() {
            return;
        }
        match self.selected_channel_message {
            None => self.selected_channel_message = msgs.last().copied(),
            Some(cur) => {
                if let Some(pos) = msgs.iter().position(|&i| i == cur) {
                    if pos > 0 {
                        self.selected_channel_message = Some(msgs[pos - 1]);
                    }
                }
            }
        }
    }

    pub fn select_channel_message_down(&mut self) {
        let msgs: Vec<usize> = self
            .channel_messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_user_message())
            .map(|(i, _)| i)
            .collect();
        if msgs.is_empty() {
            return;
        }
        match self.selected_channel_message {
            None => return,
            Some(cur) => {
                if let Some(pos) = msgs.iter().position(|&i| i == cur) {
                    if pos + 1 < msgs.len() {
                        self.selected_channel_message = Some(msgs[pos + 1]);
                    } else {
                        self.selected_channel_message = None;
                    }
                }
            }
        }
    }

    pub fn selected_channel_message_id(&self) -> Option<&str> {
        self.selected_channel_message
            .and_then(|idx| self.channel_messages.get(idx))
            .map(|m| m.id.as_str())
    }

    // ---- Reaction picker ----

    pub fn open_reaction_picker(&mut self) {
        let has_selection = match self.view_mode {
            ViewMode::Chats => self.selected_message.is_some(),
            ViewMode::Teams => self.selected_channel_message.is_some(),
        };
        if has_selection {
            self.dialog = DialogMode::ReactionPicker;
            self.selected_reaction = 0;
        }
    }

    pub fn close_dialog(&mut self) {
        self.dialog = DialogMode::None;
    }

    pub fn show_error(&mut self, title: &str, message: &str, details: &str) {
        self.dialog = DialogMode::Error(ErrorInfo {
            title: title.to_string(),
            message: message.to_string(),
            details: details.to_string(),
        });
    }

    // ---- Presence picker ----

    pub fn open_presence_picker(&mut self) {
        self.dialog = DialogMode::PresencePicker;
        self.selected_presence = 0;
    }

    // ---- Teams navigation ----

    pub fn switch_to_chats(&mut self) {
        self.view_mode = ViewMode::Chats;
    }

    pub fn switch_to_teams(&mut self) {
        self.view_mode = ViewMode::Teams;
    }

    pub fn select_next_team(&mut self) {
        if !self.teams.is_empty() {
            self.selected_team = (self.selected_team + 1).min(self.teams.len() - 1);
        }
    }

    pub fn select_prev_team(&mut self) {
        self.selected_team = self.selected_team.saturating_sub(1);
    }

    pub fn selected_team_id(&self) -> Option<&str> {
        self.teams.get(self.selected_team).map(|t| t.id.as_str())
    }

    pub fn selected_team_name(&self) -> String {
        self.teams
            .get(self.selected_team)
            .map(|t| t.display_name.clone())
            .unwrap_or_else(|| "No team selected".to_string())
    }

    pub fn select_next_channel(&mut self) {
        if !self.channels.is_empty() {
            self.selected_channel = (self.selected_channel + 1).min(self.channels.len() - 1);
        }
    }

    pub fn select_prev_channel(&mut self) {
        self.selected_channel = self.selected_channel.saturating_sub(1);
    }

    pub fn selected_channel_id(&self) -> Option<&str> {
        self.channels
            .get(self.selected_channel)
            .map(|c| c.id.as_str())
    }

    pub fn selected_channel_name(&self) -> String {
        self.channels
            .get(self.selected_channel)
            .map(|c| format!("# {}", c.display_name))
            .unwrap_or_else(|| "No channel selected".to_string())
    }

    pub fn next_teams_panel(&mut self) {
        self.teams_panel = match self.teams_panel {
            TeamsPanel::TeamList => TeamsPanel::ChannelList,
            TeamsPanel::ChannelList => TeamsPanel::ChannelMessages,
            TeamsPanel::ChannelMessages => TeamsPanel::ChannelInput,
            TeamsPanel::ChannelInput => TeamsPanel::TeamList,
        };
    }

    pub fn prev_teams_panel(&mut self) {
        self.teams_panel = match self.teams_panel {
            TeamsPanel::TeamList => TeamsPanel::ChannelInput,
            TeamsPanel::ChannelList => TeamsPanel::TeamList,
            TeamsPanel::ChannelMessages => TeamsPanel::ChannelList,
            TeamsPanel::ChannelInput => TeamsPanel::ChannelMessages,
        };
    }

    // ---- Channel input ----

    pub fn channel_insert_char(&mut self, c: char) {
        self.channel_input.insert(self.channel_input_cursor, c);
        self.channel_input_cursor += c.len_utf8();
    }

    pub fn channel_delete_char(&mut self) {
        if self.channel_input_cursor > 0 {
            let prev_len = self.channel_input[..self.channel_input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.channel_input_cursor -= prev_len;
            self.channel_input.remove(self.channel_input_cursor);
        }
    }

    pub fn take_channel_input(&mut self) -> String {
        let input = self.channel_input.clone();
        self.channel_input.clear();
        self.channel_input_cursor = 0;
        input
    }

    pub fn channel_move_cursor_left(&mut self) {
        if self.channel_input_cursor > 0 {
            let prev_len = self.channel_input[..self.channel_input_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.channel_input_cursor -= prev_len;
        }
    }

    pub fn channel_move_cursor_right(&mut self) {
        if self.channel_input_cursor < self.channel_input.len() {
            let next_len = self.channel_input[self.channel_input_cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.channel_input_cursor += next_len;
        }
    }

    pub fn channel_scroll_up(&mut self) {
        self.channel_scroll_offset = self.channel_scroll_offset.saturating_add(3);
    }

    pub fn channel_scroll_down(&mut self) {
        self.channel_scroll_offset = self.channel_scroll_offset.saturating_sub(3);
    }

    // ---- Unread tracking ----

    pub fn update_total_unread(&mut self) {
        self.total_unread = self.chats.iter().map(|c| c.unread_count()).sum();
    }

    /// Detect new messages and return true if there are new ones (for notification bell)
    pub fn detect_new_messages(&mut self) -> bool {
        let new_ids: HashSet<String> = self
            .messages
            .iter()
            .map(|m| m.id.clone())
            .collect();

        if self.known_message_ids.is_empty() {
            self.known_message_ids = new_ids;
            return false;
        }

        let has_new = new_ids.iter().any(|id| !self.known_message_ids.contains(id));
        self.known_message_ids = new_ids;
        has_new
    }

    // ---- Channel members ----

    pub fn toggle_members(&mut self) {
        self.show_members = !self.show_members;
    }

    // ---- Reply state ----

    pub fn start_reply(&mut self) {
        if let Some(idx) = self.selected_message {
            if let Some(msg) = self.messages.get(idx) {
                self.reply_to_message_id = Some(msg.id.clone());
                let preview: String = msg.content_text().chars().take(40).collect();
                self.reply_to_preview = format!("{}: {}", msg.sender_name(), preview);
                self.selected_message = None;
                self.active_panel = Panel::Input;
            }
        }
    }

    pub fn start_channel_reply(&mut self) {
        if let Some(idx) = self.selected_channel_message {
            if let Some(msg) = self.channel_messages.get(idx) {
                self.reply_to_message_id = Some(msg.id.clone());
                let preview: String = msg.content_text().chars().take(40).collect();
                self.reply_to_preview = format!("{}: {}", msg.sender_name(), preview);
                self.selected_channel_message = None;
                self.teams_panel = TeamsPanel::ChannelInput;
            }
        }
    }

    pub fn cancel_reply(&mut self) {
        self.reply_to_message_id = None;
        self.reply_to_preview.clear();
    }

    pub fn is_replying(&self) -> bool {
        self.reply_to_message_id.is_some() && self.editing_message_id.is_none()
    }

    // ---- Edit state ----

    pub fn start_edit(&mut self) {
        let current_uid = self.current_user_id().to_string();
        if let Some(idx) = self.selected_message {
            if let Some(msg) = self.messages.get(idx) {
                if msg.sender_id() == Some(&current_uid) {
                    self.editing_message_id = Some(msg.id.clone());
                    self.input = msg.content_text();
                    self.input_cursor = self.input.len();
                    self.selected_message = None;
                    self.active_panel = Panel::Input;
                }
            }
        }
    }

    pub fn start_channel_edit(&mut self) {
        let current_uid = self.current_user_id().to_string();
        if let Some(idx) = self.selected_channel_message {
            if let Some(msg) = self.channel_messages.get(idx) {
                if msg.sender_id() == Some(&current_uid) {
                    self.editing_message_id = Some(msg.id.clone());
                    self.channel_input = msg.content_text();
                    self.channel_input_cursor = self.channel_input.len();
                    self.selected_channel_message = None;
                    self.teams_panel = TeamsPanel::ChannelInput;
                }
            }
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing_message_id = None;
    }

    pub fn is_editing(&self) -> bool {
        self.editing_message_id.is_some()
    }

    /// Returns true if the currently selected message was sent by the current user
    pub fn is_own_selected_message(&self) -> bool {
        let uid = self.current_user_id();
        self.selected_message
            .and_then(|idx| self.messages.get(idx))
            .and_then(|m| m.sender_id())
            .map(|id| id == uid)
            .unwrap_or(false)
    }

    pub fn is_own_selected_channel_message(&self) -> bool {
        let uid = self.current_user_id();
        self.selected_channel_message
            .and_then(|idx| self.channel_messages.get(idx))
            .and_then(|m| m.sender_id())
            .map(|id| id == uid)
            .unwrap_or(false)
    }

    // ---- Pagination helpers ----

    pub fn prepend_older_messages(&mut self, older: Vec<Message>) {
        let offset_increase = older.len();
        let mut combined = older;
        combined.append(&mut self.messages);
        self.messages = combined;
        // Adjust scroll so the view doesn't jump
        self.scroll_offset = self.scroll_offset.saturating_add(offset_increase * 2);
    }

    pub fn prepend_older_channel_messages(&mut self, older: Vec<Message>) {
        let offset_increase = older.len();
        let mut combined = older;
        combined.append(&mut self.channel_messages);
        self.channel_messages = combined;
        self.channel_scroll_offset = self.channel_scroll_offset.saturating_add(offset_increase * 2);
    }

    // ---- Cache helpers ----

    /// Show cached channels for the currently selected team (instant, no API call)
    pub fn show_cached_channels_for_selected_team(&mut self) {
        if let Some(tid) = self.selected_team_id().map(String::from) {
            if let Some(cached) = self.channels_cache.get(&tid) {
                self.channels = cached.clone();
                self.selected_channel = 0;
                self.channel_scroll_offset = 0;
                // Also show cached messages for first channel
                self.show_cached_messages_for_selected_channel();
            } else {
                self.channels.clear();
                self.channel_messages.clear();
            }
        }
    }

    /// Show cached messages for the currently selected channel (instant)
    pub fn show_cached_messages_for_selected_channel(&mut self) {
        if let Some(ch_id) = self.selected_channel_id().map(String::from) {
            if let Some(cached) = self.channel_message_cache.get(&ch_id) {
                self.channel_messages = cached.clone();
                self.channel_scroll_offset = 0;
            } else {
                self.channel_messages.clear();
            }
        }
    }
}
