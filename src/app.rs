use crate::models::{Chat, Message, User};

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

#[derive(Debug, Clone)]
pub enum AppScreen {
    Loading { message: String },
    Main,
    Error { message: String },
}

pub struct App {
    pub screen: AppScreen,
    pub active_panel: Panel,
    pub chats: Vec<Chat>,
    pub selected_chat: usize,
    pub messages: Vec<Message>,
    pub input: String,
    pub input_cursor: usize,
    pub current_user: Option<User>,
    pub status_message: String,
    pub scroll_offset: usize,
    pub last_refresh: std::time::Instant,
    pub refresh_interval: std::time::Duration,
    pub new_chat_mode: bool,
    pub new_chat_input: String,
    pub new_chat_cursor: usize,
    pub suggestions: Vec<UserSuggestion>,
    pub selected_suggestion: usize,
    pub last_search_query: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Loading {
                message: "Starting...".to_string(),
            },
            active_panel: Panel::ChatList,
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

    pub fn enter_new_chat_mode(&mut self) {
        self.new_chat_mode = true;
        self.new_chat_input.clear();
        self.new_chat_cursor = 0;
    }

    pub fn exit_new_chat_mode(&mut self) {
        self.new_chat_mode = false;
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
}
