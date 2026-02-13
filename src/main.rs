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

    let config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}\n", e);
            config::print_setup_guide();
            return Ok(());
        }
    };

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
        if let Ok(messages) = graph.get_messages(&chat_id).await {
            app.messages = messages;
            app.detect_new_messages(); // Initialize tracking
        }
    }

    // Fetch initial presence (best-effort)
    load_presence(&graph, &mut app).await;

    app.mark_refreshed();

    // Main event loop
    loop {
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
                match app.dialog {
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
                            load_teams(&graph, &mut app).await;
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
                    ViewMode::Teams => handle_teams_keys(&mut app, &graph, key.code).await,
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
                        if let Ok(messages) = graph.get_messages(&chat_id).await {
                            app.messages = messages;
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
                        if let Ok(msgs) = graph.get_channel_messages(&team_id, &channel_id).await {
                            app.channel_messages = msgs;
                        }
                    }
                }
            }

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
            KeyCode::Char('r') => load_messages(graph, app).await,
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
            KeyCode::Esc => app.active_panel = Panel::ChatList,
            KeyCode::Tab => app.next_panel(),
            KeyCode::BackTab => app.prev_panel(),
            KeyCode::Enter => {
                let msg = app.take_input();
                if !msg.is_empty() {
                    send_message(graph, app, &msg).await;
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

async fn handle_teams_keys(app: &mut app::App, graph: &client::GraphClient, code: KeyCode) {
    match app.teams_panel {
        TeamsPanel::TeamList => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = app.selected_team;
                app.select_prev_team();
                if prev != app.selected_team {
                    load_channels(graph, app).await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let prev = app.selected_team;
                app.select_next_team();
                if prev != app.selected_team {
                    load_channels(graph, app).await;
                }
            }
            KeyCode::Enter => {
                load_channels(graph, app).await;
                app.teams_panel = TeamsPanel::ChannelList;
            }
            KeyCode::Char('r') => load_teams(graph, app).await,
            _ => {}
        },
        TeamsPanel::ChannelList => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = app.selected_channel;
                app.select_prev_channel();
                if prev != app.selected_channel {
                    load_channel_messages(graph, app).await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let prev = app.selected_channel;
                app.select_next_channel();
                if prev != app.selected_channel {
                    load_channel_messages(graph, app).await;
                }
            }
            KeyCode::Enter => {
                load_channel_messages(graph, app).await;
                app.teams_panel = TeamsPanel::ChannelMessages;
            }
            KeyCode::Esc => app.teams_panel = TeamsPanel::TeamList,
            _ => {}
        },
        TeamsPanel::ChannelMessages => match code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Up | KeyCode::Char('k') => app.channel_scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => app.channel_scroll_down(),
            KeyCode::Char('r') => load_channel_messages(graph, app).await,
            KeyCode::Enter => app.teams_panel = TeamsPanel::ChannelInput,
            KeyCode::Esc => app.teams_panel = TeamsPanel::ChannelList,
            _ => {}
        },
        TeamsPanel::ChannelInput => match code {
            KeyCode::Esc => app.teams_panel = TeamsPanel::ChannelMessages,
            KeyCode::Tab => app.next_teams_panel(),
            KeyCode::BackTab => app.prev_teams_panel(),
            KeyCode::Enter => {
                let msg = app.take_channel_input();
                if !msg.is_empty() {
                    send_channel_message(graph, app, &msg).await;
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
            if let (Some(chat_id), Some(msg_id)) = (
                app.selected_chat_id().map(String::from),
                app.selected_message_id().map(String::from),
            ) {
                let (reaction_type, _) = models::REACTION_TYPES[app.selected_reaction];
                match graph.set_reaction(&chat_id, &msg_id, reaction_type).await {
                    Ok(_) => {
                        app.status_message = format!("Reacted with {}", models::REACTION_TYPES[app.selected_reaction].1);
                        app.close_dialog();
                        load_messages(graph, app).await;
                    }
                    Err(e) => {
                        app.status_message = format!("React failed: {}", e);
                        app.close_dialog();
                    }
                }
            } else {
                app.close_dialog();
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
                }
                Err(e) => {
                    app.status_message = format!("Set status failed: {}", e);
                }
            }
            app.close_dialog();
        }
        _ => {}
    }
}

// ---- Data loading helpers ----

async fn load_messages(graph: &client::GraphClient, app: &mut app::App) {
    app.scroll_offset = 0;
    app.selected_message = None;
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.get_messages(&chat_id).await {
            Ok(messages) => {
                app.messages = messages;
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

async fn send_message(graph: &client::GraphClient, app: &mut app::App, content: &str) {
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.send_message(&chat_id, content).await {
            Ok(_) => {
                app.status_message = "Message sent".to_string();
                load_messages(graph, app).await;
            }
            Err(e) => app.status_message = format!("Send failed: {}", e),
        }
    }
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
        Err(e) => app.status_message = format!("Failed: {}", e),
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
    // Refresh presence too
    load_presence(graph, app).await;
    app.mark_refreshed();
}

async fn load_presence(graph: &client::GraphClient, app: &mut app::App) {
    // Load own presence
    if let Ok(p) = graph.get_my_presence().await {
        app.my_presence = p.availability.unwrap_or_else(|| "PresenceUnknown".to_string());
    }

    // Load presence for chat members
    let mut user_ids: Vec<String> = Vec::new();
    for chat in &app.chats {
        if let Some(ref members) = chat.members {
            for m in members {
                if let Some(ref uid) = m.user_id {
                    if uid != app.current_user_id() && !user_ids.contains(uid) {
                        user_ids.push(uid.clone());
                    }
                }
            }
        }
    }

    // Batch presence query (max 650 per call)
    for chunk in user_ids.chunks(650) {
        if let Ok(presences) = graph.get_presences(&chunk.to_vec()).await {
            for p in presences {
                if let Some(avail) = p.availability {
                    app.presence_map.insert(p.id.clone(), avail);
                }
            }
        }
    }
}

async fn load_teams(graph: &client::GraphClient, app: &mut app::App) {
    app.status_message = "Loading teams...".to_string();
    match graph.list_teams().await {
        Ok(teams) => {
            app.teams = teams;
            if !app.teams.is_empty() {
                app.selected_team = 0;
                load_channels(graph, app).await;
            }
            app.status_message.clear();
        }
        Err(e) => app.status_message = format!("Teams: {}", e),
    }
}

async fn load_channels(graph: &client::GraphClient, app: &mut app::App) {
    if let Some(team_id) = app.selected_team_id().map(String::from) {
        match graph.list_channels(&team_id).await {
            Ok(channels) => {
                app.channels = channels;
                app.selected_channel = 0;
                app.channel_messages.clear();
                app.channel_scroll_offset = 0;
                if !app.channels.is_empty() {
                    load_channel_messages(graph, app).await;
                }
            }
            Err(e) => app.status_message = format!("Channels: {}", e),
        }
    }
}

async fn load_channel_messages(graph: &client::GraphClient, app: &mut app::App) {
    app.channel_scroll_offset = 0;
    if let (Some(team_id), Some(channel_id)) = (
        app.selected_team_id().map(String::from),
        app.selected_channel_id().map(String::from),
    ) {
        match graph.get_channel_messages(&team_id, &channel_id).await {
            Ok(msgs) => {
                app.channel_messages = msgs;
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
                load_channel_messages(graph, app).await;
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
    println!("  --pkce      Use PKCE browser flow instead of device code flow");
    println!("  --logout    Clear stored credentials securely");
    println!("  --help, -h  Show this help");
    println!();
    println!("AUTHENTICATION:");
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
    println!("  p                Set presence status");
    println!("  r                Refresh chats and messages");
    println!("  Esc              Back to chat list / deselect message");
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
