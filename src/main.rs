mod app;
mod auth;
mod client;
mod config;
mod models;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use app::{AppScreen, DialogMode, Panel, TeamsPanel, ViewMode};

/// Simple base64 encoder for OSC 52 clipboard (no external dep needed)
fn base64_encode(input: &str) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut enc = Base64Writer::new(&mut buf);
        enc.write_all(input.as_bytes()).unwrap();
        enc.finish();
    }
    String::from_utf8(buf).unwrap()
}

struct Base64Writer<'a> {
    out: &'a mut Vec<u8>,
    buf: [u8; 3],
    pos: usize,
}

impl<'a> Base64Writer<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out, buf: [0; 3], pos: 0 }
    }
    fn finish(self) {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        if self.pos == 1 {
            let b0 = self.buf[0];
            self.out.push(CHARS[(b0 >> 2) as usize]);
            self.out.push(CHARS[((b0 & 0x03) << 4) as usize]);
            self.out.push(b'=');
            self.out.push(b'=');
        } else if self.pos == 2 {
            let (b0, b1) = (self.buf[0], self.buf[1]);
            self.out.push(CHARS[(b0 >> 2) as usize]);
            self.out.push(CHARS[((b0 & 0x03) << 4 | b1 >> 4) as usize]);
            self.out.push(CHARS[((b1 & 0x0f) << 2) as usize]);
            self.out.push(b'=');
        }
    }
}

impl<'a> std::io::Write for Base64Writer<'a> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for &byte in data {
            self.buf[self.pos] = byte;
            self.pos += 1;
            if self.pos == 3 {
                let (b0, b1, b2) = (self.buf[0], self.buf[1], self.buf[2]);
                self.out.push(CHARS[(b0 >> 2) as usize]);
                self.out.push(CHARS[((b0 & 0x03) << 4 | b1 >> 4) as usize]);
                self.out.push(CHARS[((b1 & 0x0f) << 2 | b2 >> 6) as usize]);
                self.out.push(CHARS[(b2 & 0x3f) as usize]);
                self.pos = 0;
            }
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

// Background task results delivered via channel
enum BgResult {
    Channels(String, Vec<models::Channel>),
    ChannelMessages(String, Vec<models::Message>),
    PresenceMap(std::collections::HashMap<String, String>),
    MyPresence(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--logout") {
        auth::clear_stored_tokens()?;
        println!("Credentials cleared securely from OS credential store.");
        return Ok(());
    }

    let mut config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}\n", e);
            config::print_setup_guide();
            return Ok(());
        }
    };

    // CLI override for client_id
    if let Some(pos) = args.iter().position(|a| a == "--client-id") {
        if let Some(id) = args.get(pos + 1) {
            config.client_id = id.clone();
        } else {
            eprintln!("Error: --client-id requires a value");
            return Ok(());
        }
    }

    let http_client = reqwest::Client::new();
    let use_pkce = args.iter().any(|a| a == "--pkce");
    let token = authenticate(&http_client, &config, use_pkce).await?;

    // Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config, http_client, token).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    result
}

async fn authenticate(
    client: &reqwest::Client,
    config: &config::Config,
    use_pkce: bool,
) -> Result<auth::TokenResponse> {
    if let Some(token) = auth::get_valid_token(client, config).await? {
        return Ok(token);
    }

    if use_pkce {
        println!("\n  Opening browser for sign-in (PKCE flow)...");
        let token = auth::authenticate_browser(client, config).await?;
        println!("  Authenticated! Tokens stored securely.\n");
        Ok(token)
    } else {
        println!("\n  Authentication required\n");
        let dc = auth::request_device_code(client, config).await?;
        println!("  {}\n", dc.message);
        if open::that(&dc.verification_uri).is_ok() {
            println!("  Browser opened automatically\n");
        }
        println!("  Waiting for sign-in...");
        let token = auth::poll_for_token(client, config, &dc.device_code, dc.interval).await?;
        println!("  Authenticated! Tokens stored securely.\n");
        Ok(token)
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
    http_client: reqwest::Client,
    token: auth::TokenResponse,
) -> Result<()> {
    let mut app = app::App::new();
    let mut graph = client::GraphClient::new(token.access_token.clone());

    // Background task channel for non-blocking data loading
    let (bg_tx, mut bg_rx) = tokio::sync::mpsc::unbounded_channel::<BgResult>();

    // Show loading screen
    app.screen = AppScreen::Loading {
        message: "Loading your chats...".to_string(),
    };
    terminal.draw(|f| ui::draw(f, &app))?;

    // Fetch user profile
    match graph.get_me().await {
        Ok(user) => app.current_user = Some(user),
        Err(e) => {
            app.screen = AppScreen::Error {
                message: format!("Failed to get user profile: {}", e),
            };
            terminal.draw(|f| ui::draw(f, &app))?;
            wait_for_key();
            return Ok(());
        }
    }

    // Fetch chat list
    match graph.list_chats().await {
        Ok(chats) => {
            app.chats = chats;
            app.update_total_unread();
            app.screen = AppScreen::Main;
        }
        Err(e) => {
            app.screen = AppScreen::Error {
                message: format!("Failed to load chats: {}", e),
            };
            terminal.draw(|f| ui::draw(f, &app))?;
            wait_for_key();
            return Ok(());
        }
    }

    // Load messages for the first selected chat
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        if let Ok((messages, next_link)) = graph.get_messages(&chat_id).await {
            app.messages = messages;
            app.messages_next_link = next_link;
            app.detect_new_messages(); // Initialize tracking
        }
    }

    // Fetch presence in background (non-blocking)
    spawn_presence_load(&graph, &app, &bg_tx);

    app.mark_refreshed();

    // Main event loop
    loop {
        // Process any completed background tasks (non-blocking)
        while let Ok(result) = bg_rx.try_recv() {
            match result {
                BgResult::Channels(team_id, channels) => {
                    // Cache and update display if this team is still selected
                    app.channels_cache.insert(team_id.clone(), channels.clone());
                    if app.selected_team_id() == Some(team_id.as_str()) {
                        app.channels = channels;
                        if app.selected_channel == 0 {
                            app.show_cached_messages_for_selected_channel();
                        }
                    }
                }
                BgResult::ChannelMessages(channel_id, messages) => {
                    app.channel_message_cache.insert(channel_id.clone(), messages.clone());
                    if app.selected_channel_id() == Some(channel_id.as_str())
                        && app.view_mode == ViewMode::Teams
                    {
                        app.channel_messages = messages;
                    }
                }
                BgResult::PresenceMap(map) => {
                    app.presence_map.extend(map);
                }
                BgResult::MyPresence(avail) => {
                    app.my_presence = avail;
                }
            }
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Ctrl+C always quits
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break;
                }

                // Dialog mode intercepts all keys
                match &app.dialog {
                    DialogMode::NewChat => {
                        handle_new_chat_keys(&mut app, &graph, key.code).await;
                        continue;
                    }
                    DialogMode::ReactionPicker => {
                        handle_reaction_picker_keys(&mut app, &graph, key.code).await;
                        continue;
                    }
                    DialogMode::PresencePicker => {
                        handle_presence_picker_keys(&mut app, &graph, key.code).await;
                        continue;
                    }
                    DialogMode::Error(info) => {
                        match key.code {
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                // Copy troubleshooting info to clipboard via OSC 52
                                let clip = format!(
                                    "{}\n\n{}\n\nDetails:\n{}",
                                    info.title, info.message, info.details
                                );
                                let b64 = base64_encode(&clip);
                                print!("\x1b]52;c;{}\x07", b64);
                                app.status_message = "Troubleshooting info copied to clipboard".to_string();
                                app.close_dialog();
                            }
                            KeyCode::Esc | KeyCode::Enter => {
                                app.close_dialog();
                            }
                            _ => {}
                        }
                        continue;
                    }
                    DialogMode::None => {}
                }

                // Global keys (work in both views)
                match key.code {
                    KeyCode::Char('1') if app.active_panel != Panel::Input
                        && app.teams_panel != TeamsPanel::ChannelInput =>
                    {
                        app.switch_to_chats();
                        continue;
                    }
                    KeyCode::Char('2') if app.active_panel != Panel::Input
                        && app.teams_panel != TeamsPanel::ChannelInput =>
                    {
                        if app.teams.is_empty() {
                            load_teams_with_preload(&graph, &mut app, &bg_tx).await;
                        }
                        app.switch_to_teams();
                        continue;
                    }
                    KeyCode::Char('p') if app.active_panel != Panel::Input
                        && app.teams_panel != TeamsPanel::ChannelInput =>
                    {
                        app.open_presence_picker();
                        continue;
                    }
                    _ => {}
                }

                match app.view_mode {
                    ViewMode::Chats => handle_chats_keys(&mut app, &graph, key.code).await,
                    ViewMode::Teams => {
                        handle_teams_keys(&mut app, &graph, &bg_tx, key.code).await;
                    }
                }
            }
        }

        // Auto-refresh
        if app.should_refresh() {
            if let Ok(Some(new_token)) = auth::get_valid_token(&http_client, &config).await {
                graph.set_token(new_token.access_token.clone());
            }

            // Refresh current view data
            match app.view_mode {
                ViewMode::Chats => {
                    if let Ok(chats) = graph.list_chats().await {
                        app.chats = chats;
                        app.update_total_unread();
                    }
                    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
                        if let Ok((messages, next_link)) = graph.get_messages(&chat_id).await {
                            app.messages = messages;
                            app.messages_next_link = next_link;
                            if app.detect_new_messages() {
                                // Terminal bell for new messages
                                print!("\x07");
                            }
                        }
                    }
                }
                ViewMode::Teams => {
                    if let (Some(team_id), Some(channel_id)) = (
                        app.selected_team_id().map(String::from),
                        app.selected_channel_id().map(String::from),
                    ) {
                        if let Ok((msgs, next_link)) = graph.get_channel_messages(&team_id, &channel_id).await {
                            app.channel_messages = msgs.clone();
                            app.channel_messages_next_link = next_link;
                            app.channel_message_cache.insert(channel_id, msgs);
                        }
                    }
                }
            }

            // Refresh presence in background
            spawn_presence_load(&graph, &app, &bg_tx);
            app.mark_refreshed();
        }
    }

    Ok(())
}

// ---- Chat view key handling ----

async fn handle_chats_keys(app: &mut app::App, graph: &client::GraphClient, code: KeyCode) {
    match app.active_panel {
        Panel::ChatList => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Char('n') => app.enter_new_chat_mode(),
            KeyCode::Tab => app.next_panel(),
            KeyCode::BackTab => app.prev_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = app.selected_chat;
                app.select_prev_chat();
                if prev != app.selected_chat {
                    load_messages(graph, app).await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let prev = app.selected_chat;
                app.select_next_chat();
                if prev != app.selected_chat {
                    load_messages(graph, app).await;
                }
            }
            KeyCode::Enter => app.active_panel = Panel::Input,
            KeyCode::Char('r') => refresh_all(graph, app).await,
            _ => {}
        },
        Panel::Messages => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Char('n') => app.enter_new_chat_mode(),
            KeyCode::Tab => app.next_panel(),
            KeyCode::BackTab => app.prev_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selected_message.is_some() {
                    app.select_message_up();
                } else {
                    app.scroll_messages_up();
                    // Load more messages when scrolling near the top
                    if app.scroll_offset > 0 && app.messages_next_link.is_some() && !app.loading_more_messages {
                        load_older_messages(graph, app).await;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selected_message.is_some() {
                    app.select_message_down();
                } else {
                    app.scroll_messages_down();
                }
            }
            KeyCode::Char('s') => {
                // Toggle message selection mode
                if app.selected_message.is_some() {
                    app.selected_message = None;
                } else {
                    app.select_message_up(); // Select last message
                }
            }
            KeyCode::Char('e') => {
                app.open_reaction_picker();
            }
            KeyCode::Char('r') => {
                if app.selected_message.is_some() {
                    app.start_reply();
                } else {
                    load_messages(graph, app).await;
                }
            }
            KeyCode::Char('d') => {
                if app.selected_message.is_some() && app.is_own_selected_message() {
                    delete_message(graph, app).await;
                }
            }
            KeyCode::Char('w') => {
                if app.selected_message.is_some() && app.is_own_selected_message() {
                    app.start_edit();
                }
            }
            KeyCode::Esc => {
                if app.selected_message.is_some() {
                    app.selected_message = None;
                } else {
                    app.active_panel = Panel::ChatList;
                }
            }
            _ => {}
        },
        Panel::Input => match code {
            KeyCode::Esc => {
                app.cancel_reply();
                app.cancel_edit();
                app.active_panel = Panel::ChatList;
            }
            KeyCode::Tab => app.next_panel(),
            KeyCode::BackTab => app.prev_panel(),
            KeyCode::Enter => {
                let msg = app.take_input();
                if !msg.is_empty() {
                    if let Some(edit_id) = app.editing_message_id.clone() {
                        edit_message(graph, app, &edit_id, &msg).await;
                    } else if let Some(reply_id) = app.reply_to_message_id.clone() {
                        send_reply(graph, app, &reply_id, &msg).await;
                    } else {
                        send_message(graph, app, &msg).await;
                    }
                }
            }
            KeyCode::Char(c) => app.insert_char(c),
            KeyCode::Backspace => app.delete_char(),
            KeyCode::Left => app.move_cursor_left(),
            KeyCode::Right => app.move_cursor_right(),
            _ => {}
        },
    }
}

// ---- Teams view key handling ----

async fn handle_teams_keys(
    app: &mut app::App,
    graph: &client::GraphClient,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
    code: KeyCode,
) {
    match app.teams_panel {
        TeamsPanel::TeamList => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_prev_team();
                app.show_cached_channels_for_selected_team();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next_team();
                app.show_cached_channels_for_selected_team();
            }
            KeyCode::Enter => {
                // Show cached channels immediately, then refresh in background
                app.show_cached_channels_for_selected_team();
                load_channels_with_preload(graph, app, bg_tx).await;
                if !app.channels.is_empty() {
                    app.teams_panel = TeamsPanel::ChannelList;
                }
            }
            KeyCode::Char('r') => load_teams_with_preload(graph, app, bg_tx).await,
            _ => {}
        },
        TeamsPanel::ChannelList => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_prev_channel();
                app.show_cached_messages_for_selected_channel();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next_channel();
                app.show_cached_messages_for_selected_channel();
            }
            KeyCode::Enter => {
                app.show_cached_messages_for_selected_channel();
                load_channel_messages_cached(graph, app).await;
                app.teams_panel = TeamsPanel::ChannelMessages;
            }
            KeyCode::Esc => app.teams_panel = TeamsPanel::TeamList,
            _ => {}
        },
        TeamsPanel::ChannelMessages => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selected_channel_message.is_some() {
                    app.select_channel_message_up();
                } else {
                    app.channel_scroll_up();
                    if app.channel_scroll_offset > 0 && app.channel_messages_next_link.is_some() && !app.loading_more_messages {
                        load_older_channel_messages(graph, app).await;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selected_channel_message.is_some() {
                    app.select_channel_message_down();
                } else {
                    app.channel_scroll_down();
                }
            }
            KeyCode::Char('s') => {
                if app.selected_channel_message.is_some() {
                    app.selected_channel_message = None;
                } else {
                    app.select_channel_message_up();
                }
            }
            KeyCode::Char('e') => {
                app.open_reaction_picker();
            }
            KeyCode::Char('r') => {
                if app.selected_channel_message.is_some() {
                    app.start_channel_reply();
                } else {
                    load_channel_messages_cached(graph, app).await;
                }
            }
            KeyCode::Char('d') => {
                if app.selected_channel_message.is_some() && app.is_own_selected_channel_message() {
                    delete_channel_message(graph, app).await;
                }
            }
            KeyCode::Char('w') => {
                if app.selected_channel_message.is_some() && app.is_own_selected_channel_message() {
                    app.start_channel_edit();
                }
            }
            KeyCode::Enter => app.teams_panel = TeamsPanel::ChannelInput,
            KeyCode::Esc => {
                if app.selected_channel_message.is_some() {
                    app.selected_channel_message = None;
                } else {
                    app.teams_panel = TeamsPanel::ChannelList;
                }
            }
            _ => {}
        },
        TeamsPanel::ChannelInput => match code {
            KeyCode::Esc => {
                app.cancel_reply();
                app.cancel_edit();
                app.teams_panel = TeamsPanel::ChannelMessages;
            }
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Enter => {
                let msg = app.take_channel_input();
                if !msg.is_empty() {
                    if app.editing_message_id.is_some() {
                        // Channel message editing not supported by Graph API v1.0
                        app.status_message = "Channel message editing not supported".to_string();
                        app.cancel_edit();
                    } else if let Some(reply_id) = app.reply_to_message_id.clone() {
                        send_channel_reply(graph, app, &reply_id, &msg).await;
                    } else {
                        send_channel_message(graph, app, &msg).await;
                    }
                }
            }
            KeyCode::Char(c) => app.channel_insert_char(c),
            KeyCode::Backspace => app.channel_delete_char(),
            KeyCode::Left => app.channel_move_cursor_left(),
            KeyCode::Right => app.channel_move_cursor_right(),
            _ => {}
        },
    }
}

// ---- Dialog key handling ----

async fn handle_new_chat_keys(
    app: &mut app::App,
    graph: &client::GraphClient,
    code: KeyCode,
) {
    match code {
        KeyCode::Esc => app.exit_new_chat_mode(),
        KeyCode::Enter => {
            let email = if !app.suggestions.is_empty() {
                app.select_suggestion().unwrap_or_default()
            } else {
                app.take_new_chat_input()
            };
            if !email.is_empty() {
                create_new_chat(graph, app, &email).await;
            }
        }
        KeyCode::Up => app.suggestion_up(),
        KeyCode::Down => app.suggestion_down(),
        KeyCode::Tab => {
            if let Some(s) = app.suggestions.get(app.selected_suggestion) {
                app.new_chat_input = s.email.clone();
                app.new_chat_cursor = app.new_chat_input.len();
                app.suggestions.clear();
            }
        }
        KeyCode::Char(c) => {
            app.new_chat_insert_char(c);
            app.selected_suggestion = 0;
        }
        KeyCode::Backspace => {
            app.new_chat_delete_char();
            app.selected_suggestion = 0;
            if app.new_chat_input.len() < 2 {
                app.suggestions.clear();
            }
        }
        _ => {}
    }

    // Trigger user search
    if app.new_chat_mode && app.should_search() {
        let query = app.new_chat_input.clone();
        app.last_search_query = query.clone();
        match graph.search_users(&query).await {
            Ok(users) => {
                app.suggestions = users
                    .into_iter()
                    .map(|u| app::UserSuggestion {
                        display_name: u.display_name,
                        email: u.mail.or(u.user_principal_name).unwrap_or_default(),
                        id: u.id,
                    })
                    .filter(|s| !s.email.is_empty())
                    .collect();
                app.selected_suggestion = 0;
            }
            Err(_) => app.suggestions.clear(),
        }
    }
}

async fn handle_reaction_picker_keys(
    app: &mut app::App,
    graph: &client::GraphClient,
    code: KeyCode,
) {
    match code {
        KeyCode::Esc => app.close_dialog(),
        KeyCode::Left => {
            app.selected_reaction = app.selected_reaction.saturating_sub(1);
        }
        KeyCode::Right => {
            let max = models::REACTION_TYPES.len().saturating_sub(1);
            app.selected_reaction = (app.selected_reaction + 1).min(max);
        }
        KeyCode::Enter => {
            let (reaction_type, label) = models::REACTION_TYPES[app.selected_reaction];

            match app.view_mode {
                ViewMode::Chats => {
                    if let (Some(chat_id), Some(msg_id)) = (
                        app.selected_chat_id().map(String::from),
                        app.selected_message_id().map(String::from),
                    ) {
                        match graph.set_reaction(&chat_id, &msg_id, reaction_type).await {
                            Ok(_) => {
                                app.status_message = format!("Reacted with {}", label);
                                app.close_dialog();
                                load_messages(graph, app).await;
                            }
                            Err(e) => {
                                app.show_error(
                                    "Reaction Failed",
                                    &format!("Could not add {} reaction.", label),
                                    &format!(
                                        "Chat: {}\nMessage: {}\nReaction: {}\nError: {}",
                                        chat_id, msg_id, label, e
                                    ),
                                );
                            }
                        }
                    } else {
                        app.close_dialog();
                    }
                }
                ViewMode::Teams => {
                    if let (Some(team_id), Some(channel_id), Some(msg_id)) = (
                        app.selected_team_id().map(String::from),
                        app.selected_channel_id().map(String::from),
                        app.selected_channel_message_id().map(String::from),
                    ) {
                        match graph
                            .set_channel_reaction(&team_id, &channel_id, &msg_id, reaction_type)
                            .await
                        {
                            Ok(_) => {
                                app.status_message = format!("Reacted with {}", label);
                                app.close_dialog();
                                load_channel_messages_cached(graph, app).await;
                            }
                            Err(e) => {
                                app.show_error(
                                    "Reaction Failed",
                                    &format!("Could not add {} reaction.", label),
                                    &format!(
                                        "Team: {}\nChannel: {}\nMessage: {}\nReaction: {}\nError: {}",
                                        team_id, channel_id, msg_id, label, e
                                    ),
                                );
                            }
                        }
                    } else {
                        app.close_dialog();
                    }
                }
            }
        }
        _ => {}
    }
}

async fn handle_presence_picker_keys(
    app: &mut app::App,
    graph: &client::GraphClient,
    code: KeyCode,
) {
    match code {
        KeyCode::Esc => app.close_dialog(),
        KeyCode::Up => {
            app.selected_presence = app.selected_presence.saturating_sub(1);
        }
        KeyCode::Down => {
            let max = models::PRESENCE_STATUSES.len().saturating_sub(1);
            app.selected_presence = (app.selected_presence + 1).min(max);
        }
        KeyCode::Enter => {
            let (availability, _) = models::PRESENCE_STATUSES[app.selected_presence];
            // Map availability to activity
            let activity = match availability {
                "Available" => "Available",
                "Busy" => "Busy",
                "DoNotDisturb" => "DoNotDisturb",
                "Away" => "Away",
                "BeRightBack" => "BeRightBack",
                "Offline" => "OffWork",
                _ => "Available",
            };
            match graph.set_my_presence(availability, activity).await {
                Ok(_) => {
                    app.my_presence = availability.to_string();
                    app.status_message = format!("Status set to {}", availability);
                    app.close_dialog();
                }
                Err(e) => {
                    app.show_error(
                        "Set Status Failed",
                        &format!("Could not set your presence to {}.", availability),
                        &format!(
                            "Availability: {}\nActivity: {}\nEndpoint: setUserPreferredPresence\nError: {}",
                            availability, activity, e
                        ),
                    );
                }
            }
        }
        _ => {}
    }
}

// ---- Data loading helpers ----

async fn load_messages(graph: &client::GraphClient, app: &mut app::App) {
    app.scroll_offset = 0;
    app.selected_message = None;
    app.cancel_reply();
    app.cancel_edit();
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.get_messages(&chat_id).await {
            Ok((messages, next_link)) => {
                app.messages = messages;
                app.messages_next_link = next_link;
                app.detect_new_messages();
                app.status_message.clear();
            }
            Err(e) => app.status_message = format!("Error: {}", e),
        }
        // Mark chat as read (best-effort)
        let user_id = app.current_user_id().to_string();
        let _ = graph.mark_chat_read(&chat_id, &user_id).await;
    }
}

async fn load_older_messages(graph: &client::GraphClient, app: &mut app::App) {
    if let Some(next_link) = app.messages_next_link.clone() {
        app.loading_more_messages = true;
        match graph.get_messages_page(&next_link).await {
            Ok((older, next)) => {
                app.messages_next_link = next;
                app.prepend_older_messages(older);
            }
            Err(e) => app.status_message = format!("Load more: {}", e),
        }
        app.loading_more_messages = false;
    }
}

async fn load_older_channel_messages(graph: &client::GraphClient, app: &mut app::App) {
    if let Some(next_link) = app.channel_messages_next_link.clone() {
        app.loading_more_messages = true;
        match graph.get_messages_page(&next_link).await {
            Ok((older, next)) => {
                app.channel_messages_next_link = next;
                app.prepend_older_channel_messages(older);
            }
            Err(e) => app.status_message = format!("Load more: {}", e),
        }
        app.loading_more_messages = false;
    }
}

async fn send_message(graph: &client::GraphClient, app: &mut app::App, content: &str) {
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.send_message(&chat_id, content).await {
            Ok(_) => {
                app.status_message = "Message sent".to_string();
                load_messages(graph, app).await;
            }
            Err(e) => {
                app.show_error(
                    "Send Failed",
                    "Could not send your message.",
                    &format!("Chat: {}\nError: {}", chat_id, e),
                );
            }
        }
    }
}

async fn send_reply(
    graph: &client::GraphClient,
    app: &mut app::App,
    reply_to_id: &str,
    content: &str,
) {
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.send_reply(&chat_id, reply_to_id, content).await {
            Ok(_) => {
                app.status_message = "Reply sent".to_string();
                app.cancel_reply();
                load_messages(graph, app).await;
            }
            Err(e) => {
                app.show_error(
                    "Reply Failed",
                    "Could not send your reply.",
                    &format!("Chat: {}\nReplyTo: {}\nError: {}", chat_id, reply_to_id, e),
                );
            }
        }
    }
}

async fn edit_message(
    graph: &client::GraphClient,
    app: &mut app::App,
    message_id: &str,
    content: &str,
) {
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.update_message(&chat_id, message_id, content).await {
            Ok(_) => {
                app.status_message = "Message edited".to_string();
                app.cancel_edit();
                load_messages(graph, app).await;
            }
            Err(e) => {
                app.show_error(
                    "Edit Failed",
                    "Could not edit your message.",
                    &format!("Chat: {}\nMessage: {}\nError: {}", chat_id, message_id, e),
                );
            }
        }
    }
}

async fn delete_message(graph: &client::GraphClient, app: &mut app::App) {
    if let (Some(chat_id), Some(msg_id)) = (
        app.selected_chat_id().map(String::from),
        app.selected_message_id().map(String::from),
    ) {
        match graph.soft_delete_message(&chat_id, &msg_id).await {
            Ok(_) => {
                app.status_message = "Message deleted".to_string();
                app.selected_message = None;
                load_messages(graph, app).await;
            }
            Err(e) => {
                app.show_error(
                    "Delete Failed",
                    "Could not delete the message.",
                    &format!("Chat: {}\nMessage: {}\nError: {}", chat_id, msg_id, e),
                );
            }
        }
    }
}

async fn send_channel_reply(
    graph: &client::GraphClient,
    app: &mut app::App,
    reply_to_id: &str,
    content: &str,
) {
    if let (Some(team_id), Some(channel_id)) = (
        app.selected_team_id().map(String::from),
        app.selected_channel_id().map(String::from),
    ) {
        match graph
            .reply_to_channel_message(&team_id, &channel_id, reply_to_id, content)
            .await
        {
            Ok(_) => {
                app.status_message = "Reply sent".to_string();
                app.cancel_reply();
                load_channel_messages_cached(graph, app).await;
            }
            Err(e) => {
                app.show_error(
                    "Reply Failed",
                    "Could not send your reply.",
                    &format!(
                        "Team: {}\nChannel: {}\nReplyTo: {}\nError: {}",
                        team_id, channel_id, reply_to_id, e
                    ),
                );
            }
        }
    }
}

async fn delete_channel_message(_graph: &client::GraphClient, app: &mut app::App) {
    // Channel message deletion is not supported via Graph API v1.0 for user-context
    app.status_message = "Channel message deletion not supported".to_string();
    app.selected_channel_message = None;
}

async fn create_new_chat(graph: &client::GraphClient, app: &mut app::App, email: &str) {
    let my_id = app.current_user_id().to_string();
    app.status_message = format!("Creating chat with {}...", email);

    match graph.create_chat(email, &my_id).await {
        Ok(new_chat) => {
            let new_id = new_chat.id.clone();
            match graph.list_chats().await {
                Ok(chats) => {
                    let idx = chats.iter().position(|c| c.id == new_id).unwrap_or(0);
                    app.chats = chats;
                    app.update_total_unread();
                    app.selected_chat = idx;
                    load_messages(graph, app).await;
                    app.active_panel = Panel::Input;
                    app.status_message = format!("Chat with {} ready", email);
                }
                Err(_) => {
                    app.status_message = "Chat created, press r to refresh".to_string();
                }
            }
        }
        Err(e) => {
            app.show_error(
                "Create Chat Failed",
                &format!("Could not create a chat with {}.", email),
                &format!("Recipient: {}\nError: {}", email, e),
            );
        }
    }
}

async fn refresh_all(graph: &client::GraphClient, app: &mut app::App) {
    app.status_message = "Refreshing...".to_string();
    match graph.list_chats().await {
        Ok(chats) => {
            app.chats = chats;
            app.update_total_unread();
            load_messages(graph, app).await;
            app.status_message = "Refreshed".to_string();
        }
        Err(e) => app.status_message = format!("Refresh failed: {}", e),
    }
    app.mark_refreshed();
}

/// Spawn presence loading as a background task (non-blocking)
fn spawn_presence_load(
    graph: &client::GraphClient,
    app: &app::App,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
) {
    let bg_graph = graph.clone_for_background();
    let tx = bg_tx.clone();
    let current_uid = app.current_user_id().to_string();
    let mut user_ids: Vec<String> = Vec::new();
    for chat in &app.chats {
        if let Some(ref members) = chat.members {
            for m in members {
                if let Some(ref uid) = m.user_id {
                    if uid != &current_uid && !user_ids.contains(uid) {
                        user_ids.push(uid.clone());
                    }
                }
            }
        }
    }

    tokio::spawn(async move {
        // Own presence
        if let Ok(p) = bg_graph.get_my_presence().await {
            if let Some(avail) = p.availability {
                let _ = tx.send(BgResult::MyPresence(avail));
            }
        }
        // Others' presence (batch, max 650 per call)
        for chunk in user_ids.chunks(650) {
            if let Ok(presences) = bg_graph.get_presences(&chunk.to_vec()).await {
                let map: std::collections::HashMap<String, String> = presences
                    .into_iter()
                    .filter_map(|p| p.availability.map(|a| (p.id, a)))
                    .collect();
                if !map.is_empty() {
                    let _ = tx.send(BgResult::PresenceMap(map));
                }
            }
        }
    });
}

async fn load_teams_with_preload(
    graph: &client::GraphClient,
    app: &mut app::App,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
) {
    app.status_message = "Loading teams...".to_string();
    match graph.list_teams().await {
        Ok(teams) => {
            app.teams = teams;
            if !app.teams.is_empty() {
                app.selected_team = 0;
                // Load first team's channels immediately
                load_channels_with_preload(graph, app, bg_tx).await;
            }
            // Spawn background preload of channels for ALL teams
            spawn_channels_preload(graph, app, bg_tx);
            app.status_message.clear();
        }
        Err(e) => app.status_message = format!("Teams: {}", e),
    }
}

/// Preload channels for all teams in background
fn spawn_channels_preload(
    graph: &client::GraphClient,
    app: &app::App,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
) {
    for team in &app.teams {
        let bg_graph = graph.clone_for_background();
        let tx = bg_tx.clone();
        let team_id = team.id.clone();
        tokio::spawn(async move {
            if let Ok(channels) = bg_graph.list_channels(&team_id).await {
                let _ = tx.send(BgResult::Channels(team_id, channels));
            }
        });
    }
}

async fn load_channels_with_preload(
    graph: &client::GraphClient,
    app: &mut app::App,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
) {
    if let Some(team_id) = app.selected_team_id().map(String::from) {
        match graph.list_channels(&team_id).await {
            Ok(channels) => {
                app.channels_cache.insert(team_id.clone(), channels.clone());
                app.channels = channels;
                app.selected_channel = 0;
                app.channel_scroll_offset = 0;
                if !app.channels.is_empty() {
                    load_channel_messages_cached(graph, app).await;
                    // Background preload messages for all other channels
                    spawn_channel_messages_preload(graph, app, &team_id, bg_tx);
                }
            }
            Err(e) => app.status_message = format!("Channels: {}", e),
        }
    }
}

/// Preload messages for all channels of a team in background
fn spawn_channel_messages_preload(
    graph: &client::GraphClient,
    app: &app::App,
    team_id: &str,
    bg_tx: &tokio::sync::mpsc::UnboundedSender<BgResult>,
) {
    let selected_ch_id = app.selected_channel_id().map(String::from);
    for ch in &app.channels {
        // Skip the already-loaded channel
        if Some(&ch.id) == selected_ch_id.as_ref() {
            continue;
        }
        let bg_graph = graph.clone_for_background();
        let tx = bg_tx.clone();
        let tid = team_id.to_string();
        let ch_id = ch.id.clone();
        tokio::spawn(async move {
            if let Ok((msgs, _)) = bg_graph.get_channel_messages(&tid, &ch_id).await {
                let _ = tx.send(BgResult::ChannelMessages(ch_id, msgs));
            }
        });
    }
}

async fn load_channel_messages_cached(graph: &client::GraphClient, app: &mut app::App) {
    app.channel_scroll_offset = 0;
    app.cancel_reply();
    app.cancel_edit();
    if let (Some(team_id), Some(channel_id)) = (
        app.selected_team_id().map(String::from),
        app.selected_channel_id().map(String::from),
    ) {
        match graph.get_channel_messages(&team_id, &channel_id).await {
            Ok((msgs, next_link)) => {
                app.channel_message_cache.insert(channel_id, msgs.clone());
                app.channel_messages = msgs;
                app.channel_messages_next_link = next_link;
                app.status_message.clear();
            }
            Err(e) => app.status_message = format!("Messages: {}", e),
        }
    }
}

async fn send_channel_message(graph: &client::GraphClient, app: &mut app::App, content: &str) {
    if let (Some(team_id), Some(channel_id)) = (
        app.selected_team_id().map(String::from),
        app.selected_channel_id().map(String::from),
    ) {
        match graph.send_channel_message(&team_id, &channel_id, content).await {
            Ok(_) => {
                app.status_message = "Channel message sent".to_string();
                load_channel_messages_cached(graph, app).await;
            }
            Err(e) => app.status_message = format!("Send failed: {}", e),
        }
    }
}

fn wait_for_key() {
    loop {
        if let Ok(true) = event::poll(std::time::Duration::from_millis(100)) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }
    }
}

fn print_help() {
    println!("ttyms - Terminal Microsoft Teams Client");
    println!();
    println!("USAGE: ttyms [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --pkce              Use PKCE browser flow instead of device code flow");
    println!("  --client-id <ID>    Override the Azure AD client ID");
    println!("  --logout            Clear stored credentials securely");
    println!("  --help, -h          Show this help");
    println!();
    println!("AUTHENTICATION:");
    println!("  Default client ID: {}", config::DEFAULT_CLIENT_ID);
    println!("  Override via --client-id or set client_id in config.toml.");
    println!();
    println!("  Default: Device Code Flow — a code is displayed, you sign in via browser.");
    println!("  --pkce:  PKCE Flow — browser opens automatically, redirects to localhost.");
    println!();
    println!("VIEWS:");
    println!("  1              Switch to Chats view");
    println!("  2              Switch to Teams & Channels view");
    println!();
    println!("KEYBOARD SHORTCUTS (Chats):");
    println!("  Tab / Shift+Tab  Switch panels (Chats → Messages → Input)");
    println!("  Up/Down or j/k   Navigate chats / scroll messages");
    println!("  Enter            Send message / select chat");
    println!("  n                New chat");
    println!("  s                Select message (in Messages panel)");
    println!("  e                React to selected message");
    println!("  r                Reply to selected / Refresh (no selection)");
    println!("  d                Delete selected message (own only)");
    println!("  w                Edit selected message (own only)");
    println!("  p                Set presence status");
    println!("  Esc              Back to chat list / deselect / cancel reply/edit");
    println!("  q                Quit");
    println!("  Ctrl+C           Force quit");
    println!();
    println!("KEYBOARD SHORTCUTS (Teams):");
    println!("  Tab / Shift+Tab  Switch panels (Teams → Channels → Messages → Input)");
    println!("  Up/Down or j/k   Navigate teams / channels / scroll messages");
    println!("  Enter            Expand team / select channel / send message");
    println!("  Esc              Go back one panel");
    println!();
    println!("SECURITY:");
    println!("  Tokens are stored in your OS credential manager:");
    println!("    Windows  - Credential Manager");
    println!("    macOS    - Keychain");
    println!("    Linux    - Secret Service (GNOME Keyring / KDE Wallet)");
    println!("  Sensitive data is zeroized in memory when no longer needed.");
}
