use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppScreen, Panel};

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.screen {
        AppScreen::Loading { message } => draw_loading(frame, message),
        AppScreen::Error { message } => draw_error(frame, message),
        AppScreen::Main => draw_main(frame, app),
    }
}

fn draw_loading(frame: &mut Frame, message: &str) {
    let area = frame.size();
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
    let area = frame.size();
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
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // Body
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);
    draw_body(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);

    if app.new_chat_mode {
        draw_new_chat_dialog(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let user_name = app
        .current_user
        .as_ref()
        .map(|u| u.display_name.as_str())
        .unwrap_or("Unknown");

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " ◆ TTYMS ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Microsoft Teams", Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(
            format!("  {} ", user_name),
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

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_chat_list(frame, app, chunks[0]);
    draw_message_area(frame, app, chunks[1]);
}

fn draw_chat_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::ChatList;
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
            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if is_selected { "▸ " } else { "  " };

            ListItem::new(Text::from(vec![
                Line::from(vec![
                    Span::styled(indicator, name_style),
                    Span::styled(name, name_style),
                ]),
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    draw_messages(frame, app, chunks[0]);
    draw_input(frame, app, chunks[1]);
}

fn draw_messages(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Messages;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let chat_name = app.selected_chat_name();
    let block = Block::default()
        .title(format!(" {} ", chat_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.messages.is_empty() {
        let empty = Paragraph::new("No messages yet")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    let current_user_id = app.current_user_id();
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        if !msg.is_user_message() {
            continue;
        }

        let is_me = msg.sender_id() == Some(current_user_id);
        let sender = msg.sender_name();
        let time = msg.formatted_time();
        let content = msg.content_text();

        let sender_style = if is_me {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(vec![
            Span::styled(sender, sender_style),
            Span::styled(
                format!("  {}", time),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        for text_line in content.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                Style::default().fg(Color::White),
            )));
        }

        lines.push(Line::from(""));
    }

    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = max_scroll.saturating_sub(app.scroll_offset.min(max_scroll));

    let paragraph = Paragraph::new(Text::from(lines)).scroll((scroll as u16, 0));
    frame.render_widget(paragraph, inner);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Input;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let display_text = if app.input.is_empty() {
        if is_active {
            "Type a message…"
        } else {
            "Press Tab → Enter to type"
        }
    } else {
        &app.input
    };

    let style = if app.input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(display_text).style(style).block(
        Block::default()
            .title(" Message ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(input, area);

    if is_active {
        let cursor_pos = app.input[..app.input_cursor].chars().count() as u16;
        frame.set_cursor(area.x + 1 + cursor_pos, area.y + 1);
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Truncate status message to fit available width
    let max_status_len = (area.width as usize).saturating_sub(55);
    let status_msg: String = app
        .status_message
        .chars()
        .take(max_status_len)
        .collect();

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Switch │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "n",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" New │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Refresh │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Nav │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            status_msg,
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(bar, area);
}

fn draw_new_chat_dialog(frame: &mut Frame, app: &App) {
    let area = frame.size();
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
    frame.set_cursor(inner.x + 2 + cursor_pos, inner.y + 1);
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
