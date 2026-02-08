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
    use app::{AppScreen, Panel};

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
        }
    }

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

                // New chat dialog intercepts all keys when active
                if app.new_chat_mode {
                    match key.code {
                        KeyCode::Esc => app.exit_new_chat_mode(),
                        KeyCode::Enter => {
                            // If suggestions exist, use selected one; otherwise use typed input
                            let email = if !app.suggestions.is_empty() {
                                app.select_suggestion().unwrap_or_default()
                            } else {
                                app.take_new_chat_input()
                            };
                            if !email.is_empty() {
                                create_new_chat(&graph, &mut app, &email).await;
                            }
                        }
                        KeyCode::Up => app.suggestion_up(),
                        KeyCode::Down => app.suggestion_down(),
                        KeyCode::Tab => {
                            // Tab fills in the selected suggestion's email
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
                    // Trigger search if input changed and >= 2 chars
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
                            Err(_) => {
                                // Search failed (likely missing scope) — user can still type email directly
                                app.suggestions.clear();
                            }
                        }
                    }
                    continue;
                }

                match app.active_panel {
                    Panel::ChatList => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('n') => app.enter_new_chat_mode(),
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        KeyCode::Up | KeyCode::Char('k') => {
                            let prev = app.selected_chat;
                            app.select_prev_chat();
                            if prev != app.selected_chat {
                                load_messages(&graph, &mut app).await;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let prev = app.selected_chat;
                            app.select_next_chat();
                            if prev != app.selected_chat {
                                load_messages(&graph, &mut app).await;
                            }
                        }
                        KeyCode::Enter => app.active_panel = Panel::Input,
                        KeyCode::Char('r') => refresh_all(&graph, &mut app).await,
                        _ => {}
                    },
                    Panel::Messages => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('n') => app.enter_new_chat_mode(),
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_messages_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_messages_down(),
                        KeyCode::Char('r') => load_messages(&graph, &mut app).await,
                        _ => {}
                    },
                    Panel::Input => match key.code {
                        KeyCode::Esc => app.active_panel = Panel::ChatList,
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        KeyCode::Enter => {
                            let msg = app.take_input();
                            if !msg.is_empty() {
                                send_message(&graph, &mut app, &msg).await;
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
        }

        // Auto-refresh
        if app.should_refresh() {
            if let Ok(Some(new_token)) = auth::get_valid_token(&http_client, &config).await {
                graph.set_token(new_token.access_token.clone());
            }
            if let Some(chat_id) = app.selected_chat_id().map(String::from) {
                if let Ok(messages) = graph.get_messages(&chat_id).await {
                    app.messages = messages;
                }
            }
            app.mark_refreshed();
        }
    }

    Ok(())
}

async fn load_messages(graph: &client::GraphClient, app: &mut app::App) {
    app.scroll_offset = 0;
    if let Some(chat_id) = app.selected_chat_id().map(String::from) {
        match graph.get_messages(&chat_id).await {
            Ok(messages) => {
                app.messages = messages;
                app.status_message.clear();
            }
            Err(e) => app.status_message = format!("Error: {}", e),
        }
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
            // Refresh chat list to get full chat data with members
            match graph.list_chats().await {
                Ok(chats) => {
                    let idx = chats.iter().position(|c| c.id == new_id).unwrap_or(0);
                    app.chats = chats;
                    app.selected_chat = idx;
                    load_messages(graph, app).await;
                    app.active_panel = app::Panel::Input;
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
            load_messages(graph, app).await;
            app.status_message = "Refreshed".to_string();
        }
        Err(e) => app.status_message = format!("Refresh failed: {}", e),
    }
    app.mark_refreshed();
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
    println!("KEYBOARD SHORTCUTS:");
    println!("  Tab / Shift+Tab  Switch panels (Chats → Messages → Input)");
    println!("  Up/Down or j/k   Navigate chats / scroll messages");
    println!("  Enter            Send message / select chat");
    println!("  n                New chat");
    println!("  r                Refresh chats and messages");
    println!("  Esc              Back to chat list from input");
    println!("  q                Quit");
    println!("  Ctrl+C           Force quit");
    println!();
    println!("SECURITY:");
    println!("  Tokens are stored in your OS credential manager:");
    println!("    Windows  - Credential Manager");
    println!("    macOS    - Keychain");
    println!("    Linux    - Secret Service (GNOME Keyring / KDE Wallet)");
    println!("  Sensitive data is zeroized in memory when no longer needed.");
}
