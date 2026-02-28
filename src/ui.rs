use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppScreen, ChatManagerTab, DialogMode, Panel, TeamsPanel, ViewMode};
use crate::models::{self, RichSegment};

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.screen {
        AppScreen::Loading { message } => draw_loading(frame, message),
        AppScreen::Error { message } => draw_error(frame, message),
        AppScreen::Main => draw_main(frame, app),
    }
}

fn draw_loading(frame: &mut Frame, message: &str) {
    let area = frame.area();
    let block = Block::default()
        .title(" ttyms ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(inner);

    let logo = Paragraph::new(vec![
        Line::from(Span::styled(
            "◆  T T Y M S",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Microsoft Teams Terminal Client",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(Color::Yellow),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(logo, chunks[1]);
}

fn draw_error(frame: &mut Frame, message: &str) {
    let area = frame.area();
    let block = Block::default()
        .title(" ttyms - Error ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Percentage(30),
        ])
        .split(inner);

    let error = Paragraph::new(message.to_string())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true });
    frame.render_widget(error, chunks[1]);

    let hint = Paragraph::new("Press any key to exit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[2]);
}

fn draw_main(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header with tabs
            Constraint::Min(10),   // Body
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);

    match app.view_mode {
        ViewMode::Chats => draw_chats_body(frame, app, chunks[1]),
        ViewMode::Teams => draw_teams_body(frame, app, chunks[1]),
    }

    draw_status_bar(frame, app, chunks[2]);

    // Draw dialogs on top
    match &app.dialog {
        DialogMode::NewChat => draw_new_chat_dialog(frame, app),
        DialogMode::ReactionPicker => draw_reaction_picker(frame, app),
        DialogMode::PresencePicker => draw_presence_picker(frame, app),
        DialogMode::Settings => draw_settings_dialog(frame, app),
        DialogMode::Search => draw_search_dialog(frame, app),
        DialogMode::ChatManager => draw_chat_manager_dialog(frame, app),
        DialogMode::CommandPalette => draw_command_palette(frame, app),
        DialogMode::FilePicker => draw_file_picker(frame, app),
        DialogMode::Error(info) => draw_error_dialog(frame, info),
        DialogMode::None => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let user_name = app
        .current_user
        .as_ref()
        .map(|u| u.display_name.as_str())
        .unwrap_or("Unknown");

    let (presence_icon, _) = models::presence_indicator(&app.my_presence);

    let chats_tab_style = if app.view_mode == ViewMode::Chats {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::Gray)
    };

    let teams_tab_style = if app.view_mode == ViewMode::Teams {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::Gray)
    };

    let unread_text = if app.total_unread > 0 {
        format!(" ({})", app.total_unread)
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " ◆ TTYMS ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled("1:", Style::default().fg(Color::DarkGray)),
        Span::styled("Chats", chats_tab_style),
        Span::styled(&unread_text, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled("2:", Style::default().fg(Color::DarkGray)),
        Span::styled("Teams", teams_tab_style),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} {} ", presence_icon, user_name),
            Style::default().fg(Color::Green),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

// ---- Chats View ----

fn draw_chats_body(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_chat_list(frame, app, chunks[0]);
    draw_message_area(frame, app, chunks[1]);
}

fn draw_chat_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::ChatList && app.view_mode == ViewMode::Chats;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" Chats ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner_height = block.inner(area).height as usize;
    let item_height = 3usize;
    let max_visible = if item_height > 0 { inner_height / item_height } else { 0 };

    let scroll_start = if max_visible > 0 && app.selected_chat >= max_visible {
        app.selected_chat - max_visible + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .chats
        .iter()
        .enumerate()
        .skip(scroll_start)
        .take(max_visible.max(1))
        .map(|(i, chat)| {
            let name = chat.display_name(app.current_user_id());
            let preview = chat.preview_text();
            let preview = if preview.chars().count() > 25 {
                format!("{}…", preview.chars().take(24).collect::<String>())
            } else {
                preview
            };

            let is_selected = i == app.selected_chat;
            let unread = chat.unread_count();
            let has_unread = unread > 0;

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if has_unread {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if is_selected { "▸ " } else { "  " };

            // Build name line with optional unread badge
            let mut name_spans = vec![
                Span::styled(indicator, name_style),
                Span::styled(name, name_style),
            ];
            if has_unread {
                name_spans.push(Span::styled(
                    format!(" ({})", unread),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ));
            }

            // Show presence indicator for 1:1 chats
            if chat.chat_type == "oneOnOne" {
                if let Some(ref members) = chat.members {
                    for m in members {
                        if m.user_id.as_deref() != Some(app.current_user_id()) {
                            if let Some(ref uid) = m.user_id {
                                if let Some(avail) = app.presence_map.get(uid) {
                                    let (icon, _) = models::presence_indicator(avail);
                                    name_spans.insert(1, Span::styled(
                                        format!("{} ", icon),
                                        Style::default(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            ListItem::new(Text::from(vec![
                Line::from(name_spans),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(preview, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(""),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_message_area(frame: &mut Frame, app: &App, area: Rect) {
    let reply_or_edit = app.is_replying() || app.is_editing();
    let constraints = if reply_or_edit {
        vec![Constraint::Min(5), Constraint::Length(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Min(5), Constraint::Length(0), Constraint::Length(3)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_messages(frame, app, &app.messages, app.scroll_offset, app.selected_message,
                  &app.selected_chat_name(), app.active_panel == Panel::Messages,
                  app.loading_more_messages && app.messages_next_link.is_some(), chunks[0]);

    if app.is_replying() {
        let reply_line = Paragraph::new(Line::from(vec![
            Span::styled(" ↩ Replying to: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&app.reply_to_preview, Style::default().fg(Color::DarkGray)),
            Span::styled("  (Esc to cancel)", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(reply_line, chunks[1]);
    } else if app.is_editing() {
        let edit_line = Paragraph::new(Line::from(vec![
            Span::styled(" ✏ Editing message", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("  (Esc to cancel)", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(edit_line, chunks[1]);
    }

    let input_title = if app.is_editing() {
        " Edit Message "
    } else if app.is_replying() {
        " Reply "
    } else {
        " Message "
    };
    draw_input_box(frame, &app.input, app.input_cursor,
                   app.active_panel == Panel::Input, input_title, chunks[2]);
}

fn draw_messages(
    frame: &mut Frame,
    app: &App,
    messages: &[models::Message],
    scroll_offset: usize,
    selected_message: Option<usize>,
    title: &str,
    is_active: bool,
    has_more: bool,
    area: Rect,
) {
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if messages.is_empty() {
        let empty = Paragraph::new("No messages yet")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    let current_user_id = app.current_user_id();
    let mut lines: Vec<Line> = Vec::new();

    if has_more {
        lines.push(Line::from(Span::styled(
            "  ▲ Scroll up to load older messages…",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(""));
    }

    for (idx, msg) in messages.iter().enumerate() {
        if !msg.is_user_message() {
            continue;
        }

        let is_me = msg.sender_id() == Some(current_user_id);
        let sender = msg.sender_name();
        let time = msg.formatted_time();
        let is_selected = selected_message == Some(idx);

        let sender_style = if is_selected {
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if is_me {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        // Sender line with presence
        let mut sender_spans = vec![
            Span::styled(sender, sender_style),
            Span::styled(
                format!("  {}", time),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        if is_selected {
            sender_spans.push(Span::styled(
                " ◀",
                Style::default().fg(Color::Magenta),
            ));
        }

        lines.push(Line::from(sender_spans));

        // Message content with rich text
        let content_html = msg
            .body
            .as_ref()
            .and_then(|b| b.content.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");

        let rich_segments = models::parse_rich_text(content_html);
        let mut content_spans: Vec<Span> = Vec::new();
        content_spans.push(Span::raw("  "));

        for seg in &rich_segments {
            match seg {
                RichSegment::Plain(text) => {
                    content_spans.push(Span::styled(text.clone(), Style::default().fg(Color::White)));
                }
                RichSegment::Bold(text) => {
                    content_spans.push(Span::styled(
                        text.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                }
                RichSegment::Italic(text) => {
                    content_spans.push(Span::styled(
                        text.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
                    ));
                }
                RichSegment::Code(text) => {
                    content_spans.push(Span::styled(
                        format!("`{}`", text),
                        Style::default().fg(Color::Cyan).bg(Color::DarkGray),
                    ));
                }
                RichSegment::Link { text, url } => {
                    content_spans.push(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    if url != text && !url.is_empty() {
                        content_spans.push(Span::styled(
                            format!(" ({})", url),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
                RichSegment::Newline => {
                    lines.push(Line::from(content_spans.clone()));
                    content_spans.clear();
                    content_spans.push(Span::raw("  "));
                }
            }
        }

        if !content_spans.is_empty() {
            lines.push(Line::from(content_spans));
        }

        // Reactions
        let reactions = msg.reactions_summary();
        if !reactions.is_empty() {
            let mut reaction_spans: Vec<Span> = vec![Span::raw("  ")];
            for (emoji, count) in &reactions {
                reaction_spans.push(Span::styled(
                    format!(" {} {} ", emoji, count),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
                reaction_spans.push(Span::raw(" "));
            }
            lines.push(Line::from(reaction_spans));
        }

        lines.push(Line::from(""));
    }

    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = max_scroll.saturating_sub(scroll_offset.min(max_scroll));

    let paragraph = Paragraph::new(Text::from(lines)).scroll((scroll as u16, 0));
    frame.render_widget(paragraph, inner);
}

fn draw_input_box(
    frame: &mut Frame,
    input: &str,
    cursor: usize,
    is_active: bool,
    title: &str,
    area: Rect,
) {
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let display_text = if input.is_empty() {
        if is_active {
            "Type a message…"
        } else {
            "Press Tab → Enter to type"
        }
    } else {
        input
    };

    let style = if input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let widget = Paragraph::new(display_text).style(style).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);

    if is_active {
        let cursor_pos = input[..cursor.min(input.len())].chars().count() as u16;
        frame.set_cursor_position((area.x + 1 + cursor_pos, area.y + 1));
    }
}

// ---- Teams View ----

fn draw_teams_body(frame: &mut Frame, app: &App, area: Rect) {
    let constraints = if app.show_members {
        vec![
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ]
    } else {
        vec![Constraint::Percentage(30), Constraint::Percentage(70)]
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    draw_teams_sidebar(frame, app, chunks[0]);
    draw_channel_message_area(frame, app, chunks[1]);
    if app.show_members {
        draw_channel_members(frame, app, chunks[2]);
    }
}

fn draw_teams_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_team_list(frame, app, chunks[0]);
    draw_channel_list(frame, app, chunks[1]);
}

fn draw_team_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.teams_panel == TeamsPanel::TeamList && app.view_mode == ViewMode::Teams;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" Teams ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.teams.is_empty() {
        let empty = Paragraph::new("No teams found")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .teams
        .iter()
        .enumerate()
        .map(|(i, team)| {
            let is_selected = i == app.selected_team;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let indicator = if is_selected { "▸ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(&team.display_name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_channel_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.teams_panel == TeamsPanel::ChannelList && app.view_mode == ViewMode::Teams;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let title = format!(" {} — Channels ", app.selected_team_name());
    let title_short: String = title.chars().take((area.width as usize).saturating_sub(2)).collect();

    let block = Block::default()
        .title(title_short)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.channels.is_empty() {
        let empty = Paragraph::new("Select a team above")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .channels
        .iter()
        .enumerate()
        .map(|(i, channel)| {
            let is_selected = i == app.selected_channel;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let indicator = if is_selected { "▸ " } else { "  " };
            let prefix = match channel.membership_type.as_deref() {
                Some("private") => "🔒 ",
                _ => "# ",
            };
            ListItem::new(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                Span::styled(&channel.display_name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_channel_members(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" Members ({}) ", app.channel_members.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if app.channel_members.is_empty() {
        let empty = Paragraph::new("No members loaded")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .channel_members
        .iter()
        .map(|member| {
            let role_badge = if member.is_owner() { " 👑" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(member.name(), Style::default().fg(Color::White)),
                Span::styled(role_badge, Style::default().fg(Color::Yellow)),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_channel_message_area(frame: &mut Frame, app: &App, area: Rect) {
    let reply_or_edit = app.is_replying() || app.is_editing();
    let constraints = if reply_or_edit && app.view_mode == ViewMode::Teams {
        vec![Constraint::Min(5), Constraint::Length(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Min(5), Constraint::Length(0), Constraint::Length(3)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let title = app.selected_channel_name();

    if app.channel_permission_denied && app.channel_messages.is_empty() {
        let border_color = if app.teams_panel == TeamsPanel::ChannelMessages {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "⚠ Insufficient permissions",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Reading channel messages requires the ChannelMessage.Read.All",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "permission, which needs admin consent for your organization.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Ask your IT admin to grant consent for the ttyms application.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(hint, inner);
    } else {
        draw_messages(
            frame, app, &app.channel_messages, app.channel_scroll_offset,
            app.selected_channel_message,
            &title, app.teams_panel == TeamsPanel::ChannelMessages,
            app.loading_more_messages && app.channel_messages_next_link.is_some(), chunks[0],
        );
    }

    if app.view_mode == ViewMode::Teams {
        if app.is_replying() {
            let reply_line = Paragraph::new(Line::from(vec![
                Span::styled(" ↩ Replying to: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&app.reply_to_preview, Style::default().fg(Color::DarkGray)),
                Span::styled("  (Esc to cancel)", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(reply_line, chunks[1]);
        } else if app.is_editing() {
            let edit_line = Paragraph::new(Line::from(vec![
                Span::styled(" ✏ Editing message", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("  (Esc to cancel)", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(edit_line, chunks[1]);
        }
    }

    let input_title = if app.is_editing() && app.view_mode == ViewMode::Teams {
        " Edit Channel Message "
    } else if app.is_replying() && app.view_mode == ViewMode::Teams {
        " Reply "
    } else {
        " Channel Message "
    };
    draw_input_box(
        frame, &app.channel_input, app.channel_input_cursor,
        app.teams_panel == TeamsPanel::ChannelInput, input_title, chunks[2],
    );
}

// ---- Status Bar ----

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Color::DarkGray);
    let desc_style = Style::default().fg(Color::White);

    let mut spans: Vec<Span> = Vec::new();

    // Helper to add a shortcut
    let add_shortcut = |key: &str, desc: &str, spans: &mut Vec<Span>| {
        if !spans.is_empty() {
            spans.push(Span::styled(" │ ", sep_style));
        }
        spans.push(Span::styled(format!(" {} ", key), key_style));
        spans.push(Span::styled(format!(" {}", desc), desc_style));
    };

    // Context-aware shortcuts
    match app.view_mode {
        ViewMode::Chats => {
            add_shortcut("1/2", "Switch View", &mut spans);
            add_shortcut("Tab", "Switch Panel", &mut spans);
            match app.active_panel {
                Panel::ChatList => {
                    add_shortcut("n", "New Chat", &mut spans);
                    add_shortcut("g", "Manage Chat", &mut spans);
                    add_shortcut("r", "Refresh", &mut spans);
                }
                Panel::Messages => {
                    add_shortcut("s", "Select Message", &mut spans);
                    if app.selected_message.is_some() {
                        add_shortcut("r", "Reply", &mut spans);
                        add_shortcut("e", "React", &mut spans);
                        if app.is_own_selected_message() {
                            add_shortcut("w", "Edit", &mut spans);
                            add_shortcut("d", "Delete", &mut spans);
                        }
                    } else {
                        add_shortcut("e", "Add Reaction", &mut spans);
                        add_shortcut("r", "Refresh", &mut spans);
                    }
                }
                Panel::Input => {}
            }
            add_shortcut("f", "Share File", &mut spans);
            add_shortcut("/", "Search", &mut spans);
            add_shortcut("C-p", "Palette", &mut spans);
            add_shortcut("p", "Set Status", &mut spans);
            add_shortcut("o", "Settings", &mut spans);
            add_shortcut("q", "Quit", &mut spans);
        }
        ViewMode::Teams => {
            add_shortcut("1/2", "Switch View", &mut spans);
            match app.teams_panel {
                TeamsPanel::TeamList => {
                    add_shortcut("Enter", "Open Team", &mut spans);
                    add_shortcut("r", "Refresh", &mut spans);
                }
                TeamsPanel::ChannelList => {
                    add_shortcut("Enter", "Open Channel", &mut spans);
                    add_shortcut("m", "Members", &mut spans);
                    add_shortcut("Esc", "Back", &mut spans);
                }
                TeamsPanel::ChannelMessages => {
                    add_shortcut("s", "Select Message", &mut spans);
                    if app.selected_channel_message.is_some() {
                        add_shortcut("r", "Reply", &mut spans);
                        add_shortcut("e", "React", &mut spans);
                        if app.is_own_selected_channel_message() {
                            add_shortcut("d", "Delete", &mut spans);
                        }
                    } else {
                        add_shortcut("e", "Add Reaction", &mut spans);
                        add_shortcut("r", "Refresh", &mut spans);
                    }
                    add_shortcut("m", "Members", &mut spans);
                    add_shortcut("f", "Share File", &mut spans);
                    add_shortcut("Enter", "Write Message", &mut spans);
                    add_shortcut("Esc", "Back", &mut spans);
                }
                TeamsPanel::ChannelInput => {
                    add_shortcut("Enter", "Send", &mut spans);
                    add_shortcut("Esc", "Back", &mut spans);
                }
            }
            add_shortcut("/", "Search", &mut spans);
            add_shortcut("C-p", "Palette", &mut spans);
            add_shortcut("p", "Set Status", &mut spans);
            add_shortcut("o", "Settings", &mut spans);
            add_shortcut("q", "Quit", &mut spans);
        }
    }

    // Append status message if any
    let max_status_len = (area.width as usize).saturating_sub(
        spans.iter().map(|s| s.content.len()).sum::<usize>() + 4,
    );
    if !app.status_message.is_empty() && max_status_len > 5 {
        spans.push(Span::styled(" │ ", sep_style));
        let status_msg: String = app.status_message.chars().take(max_status_len).collect();
        spans.push(Span::styled(status_msg, Style::default().fg(Color::Yellow)));
    }

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(bar, area);
}

// ---- Dialogs ----

fn draw_new_chat_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let has_suggestions = !app.suggestions.is_empty();
    let dialog_height = if has_suggestions {
        7 + app.suggestions.len().min(8) as u16
    } else {
        7
    };
    let popup = centered_rect(60, dialog_height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" New Chat ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let display_input = if app.new_chat_input.is_empty() {
        "Start typing a name or email…"
    } else {
        &app.new_chat_input
    };
    let input_style = if app.new_chat_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "To:",
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::styled(display_input, input_style),
        ]),
    ];

    if has_suggestions {
        lines.push(Line::from(""));
        for (i, s) in app.suggestions.iter().enumerate() {
            let is_selected = i == app.selected_suggestion;
            let indicator = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let email_str = format!("  <{}>", s.email);
            lines.push(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(&s.display_name, style),
                Span::styled(email_str, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    lines.push(Line::from(""));
    let hint = if has_suggestions {
        "↑↓: select  │  Enter: confirm  │  Esc: cancel"
    } else {
        "Enter: create by email  │  Esc: cancel"
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    let cursor_pos = app.new_chat_input[..app.new_chat_cursor].chars().count() as u16;
    frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y + 1));
}

fn draw_reaction_picker(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = centered_rect(40, 5, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" React ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (_, emoji)) in models::REACTION_TYPES.iter().enumerate() {
        let style = if i == app.selected_reaction {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(format!(" {} ", emoji), style));
        spans.push(Span::raw(" "));
    }

    let mut lines = vec![
        Line::from(spans),
        Line::from(""),
        Line::from(Span::styled(
            "←→: select  │  Enter: react  │  Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Show which message is selected
    if let Some(idx) = app.selected_message {
        if let Some(msg) = app.messages.get(idx) {
            let preview: String = msg.content_text().chars().take(30).collect();
            lines.insert(0, Line::from(Span::styled(
                format!("On: {}…", preview),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn draw_presence_picker(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let height = models::PRESENCE_STATUSES.len() as u16 + 5;
    let popup = centered_rect(40, height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Set Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let (current_icon, current_text) = models::presence_indicator(&app.my_presence);
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Current: {} {}", current_icon, current_text),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
    ];

    for (i, (_, label)) in models::PRESENCE_STATUSES.iter().enumerate() {
        let is_selected = i == app.selected_presence;
        let indicator = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(indicator, style),
            Span::styled(*label, style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↑↓: select  │  Enter: set  │  Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn draw_settings_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = centered_rect(50, 10, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" ⚙ Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let refresh_secs = app.refresh_interval.as_secs();
    let items: Vec<(&str, String)> = vec![
        ("Refresh interval (seconds)", refresh_secs.to_string()),
    ];

    let mut lines = Vec::new();
    for (i, (label, value)) in items.iter().enumerate() {
        let is_selected = i == app.selected_setting;
        let indicator = if is_selected { "▸ " } else { "  " };

        if is_selected && app.editing_setting {
            let label_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            lines.push(Line::from(vec![
                Span::styled(indicator, label_style),
                Span::styled(format!("{}: ", label), label_style),
            ]));
            let input_display = format!("  > {}█", app.setting_input);
            lines.push(Line::from(Span::styled(
                input_display,
                Style::default().fg(Color::White),
            )));
        } else {
            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(format!("{}: ", label), style),
                Span::styled(
                    value.clone(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    let hint = if app.editing_setting {
        "Enter: save  │  Esc: cancel"
    } else {
        "↑↓: select  │  Enter: edit  │  Esc: close"
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn draw_command_palette(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let visible_count = app.palette_filtered.len().min(10);
    let dialog_height = (5 + visible_count as u16).min(area.height.saturating_sub(4));
    let popup = centered_rect(60, dialog_height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let display_input = if app.palette_input.is_empty() {
        "Type to search chats, channels, actions…"
    } else {
        &app.palette_input
    };
    let input_style = if app.palette_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let mut lines = vec![Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Magenta)),
        Span::styled(display_input, input_style),
    ])];

    if !app.palette_filtered.is_empty() {
        lines.push(Line::from(""));
        for (vi, &idx) in app.palette_filtered.iter().take(10).enumerate() {
            if let Some(item) = app.palette_items.get(idx) {
                let is_selected = vi == app.palette_selected;
                let indicator = if is_selected { "▸ " } else { "  " };
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let max_len = inner.width.saturating_sub(6) as usize;
                let label = if item.label.len() > max_len {
                    format!("{}…", &item.label[..max_len.saturating_sub(1)])
                } else {
                    item.label.clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(indicator, name_style),
                    Span::raw(format!("{} ", item.icon)),
                    Span::styled(label, name_style),
                ]));
            }
        }
    } else if !app.palette_input.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No matches",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    let cursor_pos = app.palette_input[..app.palette_cursor].chars().count() as u16;
    frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y));
}

fn draw_file_picker(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let has_error = app.file_upload_error.is_some();
    let dialog_height = if app.file_uploading {
        8
    } else if has_error {
        9
    } else {
        8
    };
    let popup = centered_rect(65, dialog_height.min(area.height.saturating_sub(4)), area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Share File ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let display_input = if app.file_path_input.is_empty() {
        "Enter file path…"
    } else {
        &app.file_path_input
    };
    let input_style = if app.file_path_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "📎 File path:",
            Style::default().fg(Color::Gray),
        )),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Green)),
            Span::styled(display_input, input_style),
        ]),
    ];

    if app.file_uploading {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "⏳ Uploading…",
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(ref err) = app.file_upload_error {
        lines.push(Line::from(""));
        let max_len = inner.width.saturating_sub(4) as usize;
        let truncated = if err.len() > max_len {
            format!("{}…", &err[..max_len.saturating_sub(1)])
        } else {
            err.clone()
        };
        lines.push(Line::from(Span::styled(
            format!("⚠ {}", truncated),
            Style::default().fg(Color::Red),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Max 4MB  │  Enter: upload & send  │  Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    if !app.file_uploading {
        let cursor_pos = app.file_path_input[..app.file_path_cursor]
            .chars()
            .count() as u16;
        frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y + 1));
    }
}

fn draw_search_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let results_count = app.search_results.len().min(10);
    let dialog_height = if app.search_loading {
        8
    } else if results_count > 0 {
        7 + (results_count as u16 * 2)
    } else if !app.search_input.is_empty() {
        8
    } else {
        7
    };
    let popup = centered_rect(70, dialog_height.min(area.height.saturating_sub(4)), area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Search Messages ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let display_input = if app.search_input.is_empty() {
        "Type to search messages…"
    } else {
        &app.search_input
    };
    let input_style = if app.search_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let mut lines = vec![
        Line::from(Span::styled("🔍 Query:", Style::default().fg(Color::Gray))),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::styled(display_input, input_style),
        ]),
    ];

    if app.search_loading {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Searching…",
            Style::default().fg(Color::Yellow),
        )));
    } else if !app.search_results.is_empty() {
        lines.push(Line::from(""));
        for (i, hit) in app.search_results.iter().take(10).enumerate() {
            let is_selected = i == app.selected_search_result;
            let indicator = if is_selected { "▸ " } else { "  " };
            let name_style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let time_str = format!("  {}", hit.formatted_time());
            lines.push(Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(hit.sender_name(), name_style),
                Span::styled(time_str, Style::default().fg(Color::DarkGray)),
            ]));
            // Truncate summary to fit
            let summary = hit.summary_text();
            let max_len = inner.width.saturating_sub(4) as usize;
            let truncated = if summary.len() > max_len {
                format!("{}…", &summary[..max_len.saturating_sub(1)])
            } else {
                summary
            };
            lines.push(Line::from(Span::styled(
                format!("    {}", truncated),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else if !app.search_input.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No results found",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    let hint = if app.search_results.is_empty() {
        "Enter: search  │  Esc: close"
    } else {
        "↑↓: select  │  Enter: open chat  │  Esc: close"
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    let cursor_pos = app.search_input[..app.search_cursor].chars().count() as u16;
    frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y + 1));
}

fn draw_chat_manager_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let content_height = match app.chat_manager_tab {
        ChatManagerTab::Members => 5 + app.chat_manager_members.len().min(10) as u16,
        ChatManagerTab::Rename => 8,
        ChatManagerTab::AddMember => {
            let sug = app.chat_manager_add_suggestions.len().min(6) as u16;
            if sug > 0 { 9 + sug } else { 8 }
        }
    };
    let dialog_height = content_height.min(area.height.saturating_sub(4));
    let popup = centered_rect(65, dialog_height, area);
    frame.render_widget(Clear, popup);

    let title = match app.chat_manager_tab {
        ChatManagerTab::Members => " Chat Members ",
        ChatManagerTab::Rename => " Rename Chat ",
        ChatManagerTab::AddMember => " Add Member ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();

    // Tab bar
    let tabs: Vec<Span> = vec![
        tab_span("1:Members", app.chat_manager_tab == ChatManagerTab::Members),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        tab_span("2:Rename", app.chat_manager_tab == ChatManagerTab::Rename),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        tab_span("3:Add", app.chat_manager_tab == ChatManagerTab::AddMember),
    ];
    lines.push(Line::from(tabs));
    lines.push(Line::from(""));

    match app.chat_manager_tab {
        ChatManagerTab::Members => {
            if app.chat_manager_loading {
                lines.push(Line::from(Span::styled(
                    "Loading members…",
                    Style::default().fg(Color::Yellow),
                )));
            } else if app.chat_manager_members.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No members found",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                let my_id = app.current_user_id().to_string();
                for (i, m) in app.chat_manager_members.iter().take(10).enumerate() {
                    let is_selected = i == app.chat_manager_selected_member;
                    let indicator = if is_selected { "▸ " } else { "  " };
                    let name = m.display_name.as_deref().unwrap_or("Unknown");
                    let is_me = m.user_id.as_deref() == Some(my_id.as_str());
                    let suffix = if is_me { " (you)" } else { "" };
                    let style = if is_selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(indicator, style),
                        Span::styled(format!("{}{}", name, suffix), style),
                    ]));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "↑↓: select  │  x: remove  │  l: leave chat  │  Esc: close",
                Style::default().fg(Color::DarkGray),
            )));
        }
        ChatManagerTab::Rename => {
            let display_input = if app.chat_manager_rename_input.is_empty() {
                "Enter new chat name…"
            } else {
                &app.chat_manager_rename_input
            };
            let input_style = if app.chat_manager_rename_input.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                "New name:",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                Span::styled(display_input, input_style),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Enter: save  │  Esc: close",
                Style::default().fg(Color::DarkGray),
            )));
        }
        ChatManagerTab::AddMember => {
            let display_input = if app.chat_manager_add_input.is_empty() {
                "Start typing a name or email…"
            } else {
                &app.chat_manager_add_input
            };
            let input_style = if app.chat_manager_add_input.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                "User:",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                Span::styled(display_input, input_style),
            ]));

            if !app.chat_manager_add_suggestions.is_empty() {
                lines.push(Line::from(""));
                for (i, s) in app.chat_manager_add_suggestions.iter().enumerate() {
                    let is_selected = i == app.chat_manager_add_selected;
                    let indicator = if is_selected { "▸ " } else { "  " };
                    let style = if is_selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let email_str = format!("  <{}>", s.email);
                    lines.push(Line::from(vec![
                        Span::styled(indicator, style),
                        Span::styled(&s.display_name, style),
                        Span::styled(email_str, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }

            lines.push(Line::from(""));
            let hint = if app.chat_manager_add_suggestions.is_empty() {
                "Enter: add by email  │  Esc: close"
            } else {
                "↑↓: select  │  Enter: add  │  Esc: close"
            };
            lines.push(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    // Set cursor for text input tabs
    match app.chat_manager_tab {
        ChatManagerTab::Rename => {
            let cursor_pos = app.chat_manager_rename_input[..app.chat_manager_rename_cursor]
                .chars()
                .count() as u16;
            frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y + 3));
        }
        ChatManagerTab::AddMember => {
            let cursor_pos = app.chat_manager_add_input[..app.chat_manager_add_cursor]
                .chars()
                .count() as u16;
            frame.set_cursor_position((inner.x + 2 + cursor_pos, inner.y + 3));
        }
        _ => {}
    }
}

fn tab_span(label: &str, active: bool) -> Span<'_> {
    if active {
        Span::styled(label, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(label, Style::default().fg(Color::DarkGray))
    }
}

fn draw_error_dialog(frame: &mut Frame, info: &crate::app::ErrorInfo) {
    let area = frame.area();

    // Calculate height based on content
    let detail_lines: Vec<&str> = info.details.lines().collect();
    let dialog_height = (6 + detail_lines.len() as u16).min(area.height.saturating_sub(4));
    let popup = centered_rect(70, dialog_height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(format!(" ⚠ {} ", info.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // message
            Constraint::Min(1),   // details
            Constraint::Length(1), // footer
        ])
        .split(inner);

    // Error message
    let msg = Paragraph::new(info.message.as_str())
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    frame.render_widget(msg, chunks[0]);

    // Troubleshooting details
    let details = Paragraph::new(info.details.as_str())
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, chunks[1]);

    // Footer with actions
    let footer = Line::from(vec![
        Span::styled(
            " C ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Copy troubleshooting info  ", Style::default().fg(Color::White)),
        Span::styled(
            " Esc ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Close", Style::default().fg(Color::White)),
    ]);
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}
