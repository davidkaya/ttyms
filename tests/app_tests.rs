//! Tests for the app module: state management, panel navigation, input handling

#[cfg(test)]
mod panel_navigation {
    use ttyms::app::{App, Panel};

    #[test]
    fn new_app_starts_on_chat_list() {
        let app = App::new();
        assert_eq!(app.active_panel, Panel::ChatList);
    }

    #[test]
    fn next_panel_cycles_forward() {
        let mut app = App::new();
        assert_eq!(app.active_panel, Panel::ChatList);
        app.next_panel();
        assert_eq!(app.active_panel, Panel::Messages);
        app.next_panel();
        assert_eq!(app.active_panel, Panel::Input);
        app.next_panel();
        assert_eq!(app.active_panel, Panel::ChatList);
    }

    #[test]
    fn prev_panel_cycles_backward() {
        let mut app = App::new();
        assert_eq!(app.active_panel, Panel::ChatList);
        app.prev_panel();
        assert_eq!(app.active_panel, Panel::Input);
        app.prev_panel();
        assert_eq!(app.active_panel, Panel::Messages);
        app.prev_panel();
        assert_eq!(app.active_panel, Panel::ChatList);
    }
}

#[cfg(test)]
mod chat_selection {
    use ttyms::app::App;
    use ttyms::models::*;

    fn make_test_chats(count: usize) -> Vec<Chat> {
        (0..count)
            .map(|i| Chat {
                id: format!("chat-{}", i),
                topic: Some(format!("Chat {}", i)),
                chat_type: "oneOnOne".to_string(),
                members: None,
                last_message_preview: None,
                unread_message_count: None,
            })
            .collect()
    }

    #[test]
    fn select_next_chat_increments() {
        let mut app = App::new();
        app.chats = make_test_chats(5);
        assert_eq!(app.selected_chat, 0);
        app.select_next_chat();
        assert_eq!(app.selected_chat, 1);
        app.select_next_chat();
        assert_eq!(app.selected_chat, 2);
    }

    #[test]
    fn select_next_chat_stops_at_end() {
        let mut app = App::new();
        app.chats = make_test_chats(3);
        app.selected_chat = 2;
        app.select_next_chat();
        assert_eq!(app.selected_chat, 2);
    }

    #[test]
    fn select_prev_chat_decrements() {
        let mut app = App::new();
        app.chats = make_test_chats(5);
        app.selected_chat = 3;
        app.select_prev_chat();
        assert_eq!(app.selected_chat, 2);
    }

    #[test]
    fn select_prev_chat_stops_at_zero() {
        let mut app = App::new();
        app.chats = make_test_chats(3);
        app.selected_chat = 0;
        app.select_prev_chat();
        assert_eq!(app.selected_chat, 0);
    }

    #[test]
    fn select_next_noop_when_no_chats() {
        let mut app = App::new();
        app.select_next_chat();
        assert_eq!(app.selected_chat, 0);
    }

    #[test]
    fn selected_chat_id_returns_correct_id() {
        let mut app = App::new();
        app.chats = make_test_chats(3);
        app.selected_chat = 1;
        assert_eq!(app.selected_chat_id(), Some("chat-1"));
    }

    #[test]
    fn selected_chat_id_none_when_empty() {
        let app = App::new();
        assert_eq!(app.selected_chat_id(), None);
    }

    #[test]
    fn selected_chat_name_returns_topic() {
        let mut app = App::new();
        app.chats = make_test_chats(3);
        app.selected_chat = 2;
        assert_eq!(app.selected_chat_name(), "Chat 2");
    }

    #[test]
    fn selected_chat_name_fallback() {
        let app = App::new();
        assert_eq!(app.selected_chat_name(), "No chat selected");
    }
}

#[cfg(test)]
mod input_handling {
    use ttyms::app::App;

    #[test]
    fn insert_char_basic() {
        let mut app = App::new();
        app.insert_char('h');
        app.insert_char('i');
        assert_eq!(app.input, "hi");
        assert_eq!(app.input_cursor, 2);
    }

    #[test]
    fn insert_char_unicode() {
        let mut app = App::new();
        app.insert_char('🎉');
        assert_eq!(app.input, "🎉");
        assert_eq!(app.input_cursor, 4); // UTF-8 length of emoji
    }

    #[test]
    fn delete_char_removes_last() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        app.delete_char();
        assert_eq!(app.input, "ab");
        assert_eq!(app.input_cursor, 2);
    }

    #[test]
    fn delete_char_noop_when_empty() {
        let mut app = App::new();
        app.delete_char();
        assert_eq!(app.input, "");
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn delete_char_unicode() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('é');
        app.delete_char();
        assert_eq!(app.input, "a");
    }

    #[test]
    fn take_input_clears_state() {
        let mut app = App::new();
        app.insert_char('t');
        app.insert_char('e');
        app.insert_char('s');
        app.insert_char('t');
        let taken = app.take_input();
        assert_eq!(taken, "test");
        assert_eq!(app.input, "");
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn cursor_movement_left() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        assert_eq!(app.input_cursor, 3);
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 2);
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn cursor_movement_left_stops_at_zero() {
        let mut app = App::new();
        app.insert_char('a');
        app.move_cursor_left();
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn cursor_movement_right() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('b');
        app.move_cursor_left();
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 0);
        app.move_cursor_right();
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn cursor_movement_right_stops_at_end() {
        let mut app = App::new();
        app.insert_char('a');
        app.move_cursor_right();
        assert_eq!(app.input_cursor, 1); // already at end
    }

    #[test]
    fn insert_at_cursor_middle() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('c');
        app.move_cursor_left();
        app.insert_char('b');
        assert_eq!(app.input, "abc");
    }

    #[test]
    fn delete_at_cursor_middle() {
        let mut app = App::new();
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        app.move_cursor_left(); // cursor after 'b'
        app.delete_char();
        assert_eq!(app.input, "ac");
    }
}

#[cfg(test)]
mod scroll_tests {
    use ttyms::app::App;

    #[test]
    fn scroll_up_increases_offset() {
        let mut app = App::new();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_messages_up();
        assert_eq!(app.scroll_offset, 3);
        app.scroll_messages_up();
        assert_eq!(app.scroll_offset, 6);
    }

    #[test]
    fn scroll_down_decreases_offset() {
        let mut app = App::new();
        app.scroll_offset = 6;
        app.scroll_messages_down();
        assert_eq!(app.scroll_offset, 3);
        app.scroll_messages_down();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn scroll_down_stops_at_zero() {
        let mut app = App::new();
        app.scroll_messages_down();
        assert_eq!(app.scroll_offset, 0);
    }
}

#[cfg(test)]
mod new_chat_mode_tests {
    use ttyms::app::{App, UserSuggestion};

    #[test]
    fn enter_and_exit_new_chat_mode() {
        let mut app = App::new();
        assert!(!app.new_chat_mode);
        app.enter_new_chat_mode();
        assert!(app.new_chat_mode);
        app.exit_new_chat_mode();
        assert!(!app.new_chat_mode);
        assert_eq!(app.new_chat_input, "");
    }

    #[test]
    fn new_chat_input_works() {
        let mut app = App::new();
        app.enter_new_chat_mode();
        app.new_chat_insert_char('a');
        app.new_chat_insert_char('b');
        assert_eq!(app.new_chat_input, "ab");
        app.new_chat_delete_char();
        assert_eq!(app.new_chat_input, "a");
    }

    #[test]
    fn take_new_chat_input_exits_mode() {
        let mut app = App::new();
        app.enter_new_chat_mode();
        app.new_chat_insert_char('x');
        let input = app.take_new_chat_input();
        assert_eq!(input, "x");
        assert!(!app.new_chat_mode);
    }

    #[test]
    fn suggestion_navigation() {
        let mut app = App::new();
        app.suggestions = vec![
            UserSuggestion {
                display_name: "Alice".into(),
                email: "alice@test.com".into(),
                id: "1".into(),
            },
            UserSuggestion {
                display_name: "Bob".into(),
                email: "bob@test.com".into(),
                id: "2".into(),
            },
            UserSuggestion {
                display_name: "Charlie".into(),
                email: "charlie@test.com".into(),
                id: "3".into(),
            },
        ];
        assert_eq!(app.selected_suggestion, 0);
        app.suggestion_down();
        assert_eq!(app.selected_suggestion, 1);
        app.suggestion_down();
        assert_eq!(app.selected_suggestion, 2);
        app.suggestion_down(); // should clamp
        assert_eq!(app.selected_suggestion, 2);
        app.suggestion_up();
        assert_eq!(app.selected_suggestion, 1);
        app.suggestion_up();
        assert_eq!(app.selected_suggestion, 0);
        app.suggestion_up(); // should clamp
        assert_eq!(app.selected_suggestion, 0);
    }

    #[test]
    fn select_suggestion_returns_email() {
        let mut app = App::new();
        app.enter_new_chat_mode();
        app.suggestions = vec![UserSuggestion {
            display_name: "Alice".into(),
            email: "alice@test.com".into(),
            id: "1".into(),
        }];
        let email = app.select_suggestion();
        assert_eq!(email, Some("alice@test.com".to_string()));
        assert!(!app.new_chat_mode);
    }

    #[test]
    fn select_suggestion_none_when_empty() {
        let mut app = App::new();
        assert_eq!(app.select_suggestion(), None);
    }

    #[test]
    fn should_search_requires_min_two_chars() {
        let mut app = App::new();
        app.enter_new_chat_mode();
        app.new_chat_insert_char('a');
        assert!(!app.should_search());
        app.new_chat_insert_char('b');
        assert!(app.should_search());
    }

    #[test]
    fn should_search_false_when_same_query() {
        let mut app = App::new();
        app.enter_new_chat_mode();
        app.new_chat_insert_char('a');
        app.new_chat_insert_char('b');
        app.last_search_query = "ab".to_string();
        assert!(!app.should_search());
    }
}

#[cfg(test)]
mod refresh_tests {
    use ttyms::app::App;

    #[test]
    fn should_refresh_after_interval() {
        let mut app = App::new();
        app.refresh_interval = std::time::Duration::from_millis(0);
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(app.should_refresh());
    }

    #[test]
    fn should_not_refresh_immediately() {
        let mut app = App::new();
        app.mark_refreshed();
        assert!(!app.should_refresh());
    }

    #[test]
    fn current_user_id_empty_when_no_user() {
        let app = App::new();
        assert_eq!(app.current_user_id(), "");
    }

    #[test]
    fn current_user_id_returns_id() {
        let mut app = App::new();
        app.current_user = Some(ttyms::models::User {
            id: "user-42".to_string(),
            display_name: "Test".to_string(),
            mail: None,
            user_principal_name: None,
        });
        assert_eq!(app.current_user_id(), "user-42");
    }
}

#[cfg(test)]
mod view_mode_tests {
    use ttyms::app::{App, ViewMode};

    #[test]
    fn default_view_mode_is_chats() {
        let app = App::new();
        assert_eq!(app.view_mode, ViewMode::Chats);
    }

    #[test]
    fn switch_to_teams() {
        let mut app = App::new();
        app.switch_to_teams();
        assert_eq!(app.view_mode, ViewMode::Teams);
    }

    #[test]
    fn switch_to_chats() {
        let mut app = App::new();
        app.switch_to_teams();
        app.switch_to_chats();
        assert_eq!(app.view_mode, ViewMode::Chats);
    }
}

#[cfg(test)]
mod teams_navigation_tests {
    use ttyms::app::{App, TeamsPanel};
    use ttyms::models::{Team, Channel};

    fn make_test_teams(count: usize) -> Vec<Team> {
        (0..count)
            .map(|i| Team {
                id: format!("team-{}", i),
                display_name: format!("Team {}", i),
                description: None,
            })
            .collect()
    }

    fn make_test_channels(count: usize) -> Vec<Channel> {
        (0..count)
            .map(|i| Channel {
                id: format!("channel-{}", i),
                display_name: format!("Channel {}", i),
                description: None,
                membership_type: Some("standard".to_string()),
            })
            .collect()
    }

    #[test]
    fn select_next_team() {
        let mut app = App::new();
        app.teams = make_test_teams(3);
        assert_eq!(app.selected_team, 0);
        app.select_next_team();
        assert_eq!(app.selected_team, 1);
    }

    #[test]
    fn select_next_team_stops_at_end() {
        let mut app = App::new();
        app.teams = make_test_teams(2);
        app.selected_team = 1;
        app.select_next_team();
        assert_eq!(app.selected_team, 1);
    }

    #[test]
    fn select_prev_team() {
        let mut app = App::new();
        app.teams = make_test_teams(3);
        app.selected_team = 2;
        app.select_prev_team();
        assert_eq!(app.selected_team, 1);
    }

    #[test]
    fn select_prev_team_stops_at_zero() {
        let mut app = App::new();
        app.teams = make_test_teams(3);
        app.select_prev_team();
        assert_eq!(app.selected_team, 0);
    }

    #[test]
    fn selected_team_id() {
        let mut app = App::new();
        app.teams = make_test_teams(3);
        app.selected_team = 1;
        assert_eq!(app.selected_team_id(), Some("team-1"));
    }

    #[test]
    fn selected_team_name() {
        let mut app = App::new();
        app.teams = make_test_teams(3);
        app.selected_team = 2;
        assert_eq!(app.selected_team_name(), "Team 2");
    }

    #[test]
    fn selected_team_name_fallback() {
        let app = App::new();
        assert_eq!(app.selected_team_name(), "No team selected");
    }

    #[test]
    fn select_next_channel() {
        let mut app = App::new();
        app.channels = make_test_channels(3);
        app.select_next_channel();
        assert_eq!(app.selected_channel, 1);
    }

    #[test]
    fn select_prev_channel() {
        let mut app = App::new();
        app.channels = make_test_channels(3);
        app.selected_channel = 2;
        app.select_prev_channel();
        assert_eq!(app.selected_channel, 1);
    }

    #[test]
    fn selected_channel_name() {
        let mut app = App::new();
        app.channels = make_test_channels(3);
        app.selected_channel = 1;
        assert_eq!(app.selected_channel_name(), "# Channel 1");
    }

    #[test]
    fn teams_panel_cycles_forward() {
        let mut app = App::new();
        assert_eq!(app.teams_panel, TeamsPanel::TeamList);
        app.next_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::ChannelList);
        app.next_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::ChannelMessages);
        app.next_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::ChannelInput);
        app.next_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::TeamList);
    }

    #[test]
    fn teams_panel_cycles_backward() {
        let mut app = App::new();
        app.prev_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::ChannelInput);
        app.prev_teams_panel();
        assert_eq!(app.teams_panel, TeamsPanel::ChannelMessages);
    }
}

#[cfg(test)]
mod message_selection_tests {
    use ttyms::app::App;
    use ttyms::models::*;

    fn make_messages() -> Vec<Message> {
        vec![
            Message {
                id: "sys1".to_string(),
                message_type: Some("systemEventMessage".to_string()),
                body: None, from: None, created_date_time: None, reactions: None,
            },
            Message {
                id: "msg1".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("First".to_string()), content_type: None }),
                from: None, created_date_time: None, reactions: None,
            },
            Message {
                id: "msg2".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("Second".to_string()), content_type: None }),
                from: None, created_date_time: None, reactions: None,
            },
            Message {
                id: "msg3".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("Third".to_string()), content_type: None }),
                from: None, created_date_time: None, reactions: None,
            },
        ]
    }

    #[test]
    fn select_message_up_selects_last_user_message() {
        let mut app = App::new();
        app.messages = make_messages();
        assert!(app.selected_message.is_none());
        app.select_message_up();
        assert_eq!(app.selected_message, Some(3)); // "msg3" at index 3
    }

    #[test]
    fn select_message_up_moves_to_previous() {
        let mut app = App::new();
        app.messages = make_messages();
        app.selected_message = Some(3);
        app.select_message_up();
        assert_eq!(app.selected_message, Some(2));
        app.select_message_up();
        assert_eq!(app.selected_message, Some(1)); // Skips system message at 0
    }

    #[test]
    fn select_message_up_stops_at_first() {
        let mut app = App::new();
        app.messages = make_messages();
        app.selected_message = Some(1);
        app.select_message_up();
        assert_eq!(app.selected_message, Some(1));
    }

    #[test]
    fn select_message_down_moves_forward() {
        let mut app = App::new();
        app.messages = make_messages();
        app.selected_message = Some(1);
        app.select_message_down();
        assert_eq!(app.selected_message, Some(2));
    }

    #[test]
    fn select_message_down_clears_at_end() {
        let mut app = App::new();
        app.messages = make_messages();
        app.selected_message = Some(3);
        app.select_message_down();
        assert!(app.selected_message.is_none());
    }

    #[test]
    fn selected_message_id() {
        let mut app = App::new();
        app.messages = make_messages();
        app.selected_message = Some(2);
        assert_eq!(app.selected_message_id(), Some("msg2"));
    }

    #[test]
    fn selected_message_id_none() {
        let app = App::new();
        assert_eq!(app.selected_message_id(), None);
    }
}

#[cfg(test)]
mod dialog_tests {
    use ttyms::app::{App, DialogMode};

    #[test]
    fn reaction_picker_requires_selected_message() {
        let mut app = App::new();
        app.open_reaction_picker();
        assert_eq!(app.dialog, DialogMode::None);
    }

    #[test]
    fn reaction_picker_opens_with_selected_message() {
        let mut app = App::new();
        app.messages = vec![ttyms::models::Message {
            id: "m1".to_string(),
            message_type: Some("message".to_string()),
            body: None, from: None, created_date_time: None, reactions: None,
        }];
        app.selected_message = Some(0);
        app.open_reaction_picker();
        assert_eq!(app.dialog, DialogMode::ReactionPicker);
    }

    #[test]
    fn presence_picker_opens() {
        let mut app = App::new();
        app.open_presence_picker();
        assert_eq!(app.dialog, DialogMode::PresencePicker);
    }

    #[test]
    fn close_dialog_clears() {
        let mut app = App::new();
        app.open_presence_picker();
        app.close_dialog();
        assert_eq!(app.dialog, DialogMode::None);
    }

    #[test]
    fn show_error_opens_error_dialog() {
        let mut app = App::new();
        app.show_error("Test Error", "Something went wrong", "Detail: xyz");
        match &app.dialog {
            DialogMode::Error(info) => {
                assert_eq!(info.title, "Test Error");
                assert_eq!(info.message, "Something went wrong");
                assert_eq!(info.details, "Detail: xyz");
            }
            _ => panic!("Expected Error dialog"),
        }
    }

    #[test]
    fn close_error_dialog() {
        let mut app = App::new();
        app.show_error("Err", "msg", "details");
        app.close_dialog();
        assert_eq!(app.dialog, DialogMode::None);
    }

    #[test]
    fn open_settings_dialog() {
        let mut app = App::new();
        app.open_settings();
        assert_eq!(app.dialog, DialogMode::Settings);
        assert_eq!(app.selected_setting, 0);
        assert!(!app.editing_setting);
    }

    #[test]
    fn close_settings_dialog() {
        let mut app = App::new();
        app.open_settings();
        app.close_dialog();
        assert_eq!(app.dialog, DialogMode::None);
    }
}

#[cfg(test)]
mod channel_input_tests {
    use ttyms::app::App;

    #[test]
    fn channel_insert_and_delete() {
        let mut app = App::new();
        app.channel_insert_char('h');
        app.channel_insert_char('i');
        assert_eq!(app.channel_input, "hi");
        app.channel_delete_char();
        assert_eq!(app.channel_input, "h");
    }

    #[test]
    fn take_channel_input_clears() {
        let mut app = App::new();
        app.channel_insert_char('x');
        let taken = app.take_channel_input();
        assert_eq!(taken, "x");
        assert_eq!(app.channel_input, "");
        assert_eq!(app.channel_input_cursor, 0);
    }

    #[test]
    fn channel_cursor_movement() {
        let mut app = App::new();
        app.channel_insert_char('a');
        app.channel_insert_char('b');
        app.channel_move_cursor_left();
        assert_eq!(app.channel_input_cursor, 1);
        app.channel_move_cursor_right();
        assert_eq!(app.channel_input_cursor, 2);
    }

    #[test]
    fn channel_scroll() {
        let mut app = App::new();
        app.channel_scroll_up();
        assert_eq!(app.channel_scroll_offset, 3);
        app.channel_scroll_down();
        assert_eq!(app.channel_scroll_offset, 0);
    }
}

#[cfg(test)]
mod unread_tracking_tests {
    use ttyms::app::App;
    use ttyms::models::*;

    #[test]
    fn update_total_unread() {
        let mut app = App::new();
        app.chats = vec![
            Chat {
                id: "c1".to_string(), topic: None, chat_type: "oneOnOne".to_string(),
                members: None, last_message_preview: None, unread_message_count: Some(3),
            },
            Chat {
                id: "c2".to_string(), topic: None, chat_type: "oneOnOne".to_string(),
                members: None, last_message_preview: None, unread_message_count: Some(2),
            },
            Chat {
                id: "c3".to_string(), topic: None, chat_type: "oneOnOne".to_string(),
                members: None, last_message_preview: None, unread_message_count: None,
            },
        ];
        app.update_total_unread();
        assert_eq!(app.total_unread, 5);
    }

    #[test]
    fn detect_new_messages_first_time() {
        let mut app = App::new();
        app.messages = vec![Message {
            id: "m1".to_string(), message_type: None, body: None,
            from: None, created_date_time: None, reactions: None,
        }];
        assert!(!app.detect_new_messages()); // First time is init
    }

    #[test]
    fn detect_new_messages_returns_true_on_new() {
        let mut app = App::new();
        app.messages = vec![Message {
            id: "m1".to_string(), message_type: None, body: None,
            from: None, created_date_time: None, reactions: None,
        }];
        app.detect_new_messages(); // Initialize

        app.messages.push(Message {
            id: "m2".to_string(), message_type: None, body: None,
            from: None, created_date_time: None, reactions: None,
        });
        assert!(app.detect_new_messages());
    }

    #[test]
    fn detect_new_messages_returns_false_when_same() {
        let mut app = App::new();
        app.messages = vec![Message {
            id: "m1".to_string(), message_type: None, body: None,
            from: None, created_date_time: None, reactions: None,
        }];
        app.detect_new_messages();
        assert!(!app.detect_new_messages());
    }
}

#[cfg(test)]
mod cache_tests {
    use ttyms::app::App;
    use ttyms::models::*;

    fn make_team(id: &str, name: &str) -> Team {
        Team {
            id: id.to_string(),
            display_name: name.to_string(),
            description: None,
        }
    }

    fn make_channel(id: &str, name: &str) -> Channel {
        Channel {
            id: id.to_string(),
            display_name: name.to_string(),
            description: None,
            membership_type: None,
        }
    }

    fn make_message(id: &str) -> Message {
        Message {
            id: id.to_string(),
            message_type: Some("message".to_string()),
            body: None,
            from: None,
            created_date_time: None,
            reactions: None,
        }
    }

    #[test]
    fn channels_cache_starts_empty() {
        let app = App::new();
        assert!(app.channels_cache.is_empty());
        assert!(app.channel_message_cache.is_empty());
    }

    #[test]
    fn show_cached_channels_updates_display() {
        let mut app = App::new();
        app.teams = vec![make_team("t1", "Team 1")];
        app.selected_team = 0;

        let channels = vec![make_channel("c1", "General"), make_channel("c2", "Random")];
        app.channels_cache.insert("t1".to_string(), channels.clone());

        app.show_cached_channels_for_selected_team();
        assert_eq!(app.channels.len(), 2);
        assert_eq!(app.channels[0].display_name, "General");
        assert_eq!(app.selected_channel, 0);
    }

    #[test]
    fn show_cached_channels_clears_when_no_cache() {
        let mut app = App::new();
        app.teams = vec![make_team("t1", "Team 1")];
        app.selected_team = 0;
        app.channels = vec![make_channel("old", "Old")];
        app.channel_messages = vec![make_message("m1")];

        // No cache for this team
        app.show_cached_channels_for_selected_team();
        assert!(app.channels.is_empty());
        assert!(app.channel_messages.is_empty());
    }

    #[test]
    fn show_cached_messages_updates_display() {
        let mut app = App::new();
        app.channels = vec![make_channel("c1", "General")];
        app.selected_channel = 0;

        let messages = vec![make_message("m1"), make_message("m2")];
        app.channel_message_cache.insert("c1".to_string(), messages);

        app.show_cached_messages_for_selected_channel();
        assert_eq!(app.channel_messages.len(), 2);
        assert_eq!(app.channel_scroll_offset, 0);
    }

    #[test]
    fn show_cached_messages_clears_when_no_cache() {
        let mut app = App::new();
        app.channels = vec![make_channel("c1", "General")];
        app.selected_channel = 0;
        app.channel_messages = vec![make_message("old")];

        // No cache for this channel
        app.show_cached_messages_for_selected_channel();
        assert!(app.channel_messages.is_empty());
    }

    #[test]
    fn cached_channels_also_show_cached_messages() {
        let mut app = App::new();
        app.teams = vec![make_team("t1", "Team 1")];
        app.selected_team = 0;

        app.channels_cache.insert("t1".to_string(), vec![make_channel("c1", "General")]);
        app.channel_message_cache.insert("c1".to_string(), vec![make_message("m1")]);

        app.show_cached_channels_for_selected_team();
        assert_eq!(app.channels.len(), 1);
        assert_eq!(app.channel_messages.len(), 1);
        assert_eq!(app.channel_messages[0].id, "m1");
    }

    #[test]
    fn navigate_teams_shows_cached_channels() {
        let mut app = App::new();
        app.teams = vec![make_team("t1", "Team 1"), make_team("t2", "Team 2")];
        app.selected_team = 0;

        app.channels_cache.insert("t1".to_string(), vec![make_channel("c1", "General")]);
        app.channels_cache.insert("t2".to_string(), vec![make_channel("c2", "Dev"), make_channel("c3", "QA")]);

        app.select_next_team();
        app.show_cached_channels_for_selected_team();
        assert_eq!(app.channels.len(), 2);
        assert_eq!(app.channels[0].display_name, "Dev");

        app.select_prev_team();
        app.show_cached_channels_for_selected_team();
        assert_eq!(app.channels.len(), 1);
        assert_eq!(app.channels[0].display_name, "General");
    }

    #[test]
    fn navigate_channels_shows_cached_messages() {
        let mut app = App::new();
        app.channels = vec![make_channel("c1", "General"), make_channel("c2", "Dev")];
        app.selected_channel = 0;

        app.channel_message_cache.insert("c1".to_string(), vec![make_message("m1")]);
        app.channel_message_cache.insert("c2".to_string(), vec![make_message("m2"), make_message("m3")]);

        app.select_next_channel();
        app.show_cached_messages_for_selected_channel();
        assert_eq!(app.channel_messages.len(), 2);

        app.select_prev_channel();
        app.show_cached_messages_for_selected_channel();
        assert_eq!(app.channel_messages.len(), 1);
    }
}

#[cfg(test)]
mod reply_tests {
    use ttyms::app::{App, Panel};
    use ttyms::models::*;

    fn make_messages_with_senders() -> Vec<Message> {
        vec![
            Message {
                id: "msg1".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("Hello world".to_string()), content_type: None }),
                from: Some(MessageFrom {
                    user: Some(MessageUser {
                        display_name: Some("Alice".to_string()),
                        id: Some("alice-id".to_string()),
                    }),
                }),
                created_date_time: None,
                reactions: None,
            },
            Message {
                id: "msg2".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("How are you?".to_string()), content_type: None }),
                from: Some(MessageFrom {
                    user: Some(MessageUser {
                        display_name: Some("Me".to_string()),
                        id: Some("my-id".to_string()),
                    }),
                }),
                created_date_time: None,
                reactions: None,
            },
        ]
    }

    #[test]
    fn start_reply_sets_context() {
        let mut app = App::new();
        app.messages = make_messages_with_senders();
        app.selected_message = Some(0);
        app.start_reply();
        assert!(app.is_replying());
        assert_eq!(app.reply_to_message_id, Some("msg1".to_string()));
        assert!(app.reply_to_preview.contains("Alice"));
        assert_eq!(app.active_panel, Panel::Input);
        assert!(app.selected_message.is_none());
    }

    #[test]
    fn cancel_reply_clears_context() {
        let mut app = App::new();
        app.messages = make_messages_with_senders();
        app.selected_message = Some(0);
        app.start_reply();
        assert!(app.is_replying());
        app.cancel_reply();
        assert!(!app.is_replying());
        assert!(app.reply_to_message_id.is_none());
        assert!(app.reply_to_preview.is_empty());
    }

    #[test]
    fn start_reply_noop_when_no_selection() {
        let mut app = App::new();
        app.messages = make_messages_with_senders();
        app.start_reply();
        assert!(!app.is_replying());
    }
}

#[cfg(test)]
mod edit_tests {
    use ttyms::app::{App, Panel};
    use ttyms::models::*;

    fn setup_app_with_own_message() -> App {
        let mut app = App::new();
        app.current_user = Some(User {
            id: "my-id".to_string(),
            display_name: "Me".to_string(),
            mail: None,
            user_principal_name: None,
        });
        app.messages = vec![
            Message {
                id: "msg1".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("Other's message".to_string()), content_type: None }),
                from: Some(MessageFrom {
                    user: Some(MessageUser {
                        display_name: Some("Alice".to_string()),
                        id: Some("alice-id".to_string()),
                    }),
                }),
                created_date_time: None,
                reactions: None,
            },
            Message {
                id: "msg2".to_string(),
                message_type: Some("message".to_string()),
                body: Some(MessageBody { content: Some("My message".to_string()), content_type: None }),
                from: Some(MessageFrom {
                    user: Some(MessageUser {
                        display_name: Some("Me".to_string()),
                        id: Some("my-id".to_string()),
                    }),
                }),
                created_date_time: None,
                reactions: None,
            },
        ];
        app
    }

    #[test]
    fn start_edit_on_own_message() {
        let mut app = setup_app_with_own_message();
        app.selected_message = Some(1); // own message
        app.start_edit();
        assert!(app.is_editing());
        assert_eq!(app.editing_message_id, Some("msg2".to_string()));
        assert_eq!(app.input, "My message");
        assert_eq!(app.active_panel, Panel::Input);
    }

    #[test]
    fn start_edit_noop_on_others_message() {
        let mut app = setup_app_with_own_message();
        app.selected_message = Some(0); // other's message
        app.start_edit();
        assert!(!app.is_editing());
        assert!(app.editing_message_id.is_none());
    }

    #[test]
    fn cancel_edit_clears_state() {
        let mut app = setup_app_with_own_message();
        app.selected_message = Some(1);
        app.start_edit();
        assert!(app.is_editing());
        app.cancel_edit();
        assert!(!app.is_editing());
    }

    #[test]
    fn is_own_selected_message_true() {
        let mut app = setup_app_with_own_message();
        app.selected_message = Some(1);
        assert!(app.is_own_selected_message());
    }

    #[test]
    fn is_own_selected_message_false() {
        let mut app = setup_app_with_own_message();
        app.selected_message = Some(0);
        assert!(!app.is_own_selected_message());
    }

    #[test]
    fn is_own_selected_message_false_when_none() {
        let app = setup_app_with_own_message();
        assert!(!app.is_own_selected_message());
    }
}

#[cfg(test)]
mod pagination_tests {
    use ttyms::app::App;
    use ttyms::models::*;

    fn make_message(id: &str) -> Message {
        Message {
            id: id.to_string(),
            message_type: Some("message".to_string()),
            body: Some(MessageBody { content: Some(format!("Msg {}", id)), content_type: None }),
            from: None,
            created_date_time: None,
            reactions: None,
        }
    }

    #[test]
    fn prepend_older_messages() {
        let mut app = App::new();
        app.messages = vec![make_message("new1"), make_message("new2")];
        let older = vec![make_message("old1"), make_message("old2")];
        app.prepend_older_messages(older);
        assert_eq!(app.messages.len(), 4);
        assert_eq!(app.messages[0].id, "old1");
        assert_eq!(app.messages[1].id, "old2");
        assert_eq!(app.messages[2].id, "new1");
        assert_eq!(app.messages[3].id, "new2");
    }

    #[test]
    fn prepend_adjusts_scroll_offset() {
        let mut app = App::new();
        app.messages = vec![make_message("new1")];
        app.scroll_offset = 5;
        let older = vec![make_message("old1"), make_message("old2"), make_message("old3")];
        app.prepend_older_messages(older);
        // scroll_offset should increase by older.len() * 2 = 6
        assert_eq!(app.scroll_offset, 11);
    }

    #[test]
    fn prepend_older_channel_messages() {
        let mut app = App::new();
        app.channel_messages = vec![make_message("new1")];
        let older = vec![make_message("old1"), make_message("old2")];
        app.prepend_older_channel_messages(older);
        assert_eq!(app.channel_messages.len(), 3);
        assert_eq!(app.channel_messages[0].id, "old1");
        assert_eq!(app.channel_messages[2].id, "new1");
    }

    #[test]
    fn messages_next_link_initially_none() {
        let app = App::new();
        assert!(app.messages_next_link.is_none());
        assert!(app.channel_messages_next_link.is_none());
        assert!(!app.loading_more_messages);
    }
}

#[cfg(test)]
mod channel_members_state {
    use ttyms::app::App;
    use ttyms::models::ChannelMember;

    fn make_member(name: &str, owner: bool) -> ChannelMember {
        ChannelMember {
            id: Some(format!("id-{}", name)),
            display_name: Some(name.to_string()),
            roles: if owner {
                vec!["owner".to_string()]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn initially_hidden_and_empty() {
        let app = App::new();
        assert!(!app.show_members);
        assert!(app.channel_members.is_empty());
    }

    #[test]
    fn toggle_members_shows_panel() {
        let mut app = App::new();
        app.toggle_members();
        assert!(app.show_members);
    }

    #[test]
    fn toggle_members_twice_hides_panel() {
        let mut app = App::new();
        app.toggle_members();
        app.toggle_members();
        assert!(!app.show_members);
    }

    #[test]
    fn channel_members_can_be_set() {
        let mut app = App::new();
        app.channel_members = vec![make_member("Alice", true), make_member("Bob", false)];
        assert_eq!(app.channel_members.len(), 2);
        assert!(app.channel_members[0].is_owner());
        assert!(!app.channel_members[1].is_owner());
    }

    #[test]
    fn channel_members_cleared_on_reassign() {
        let mut app = App::new();
        app.channel_members = vec![make_member("Alice", true)];
        app.channel_members.clear();
        assert!(app.channel_members.is_empty());
    }

    #[test]
    fn channel_permission_denied_starts_false() {
        let app = App::new();
        assert!(!app.channel_permission_denied);
    }

    #[test]
    fn channel_permission_denied_can_be_set() {
        let mut app = App::new();
        app.channel_permission_denied = true;
        assert!(app.channel_permission_denied);
        assert!(app.channel_messages.is_empty());
    }
}

#[cfg(test)]
mod delta_sync_tests {
    use ttyms::app::App;
    use ttyms::models::*;

    fn make_message_with_time(id: &str, time: &str) -> Message {
        Message {
            id: id.to_string(),
            message_type: Some("message".to_string()),
            body: Some(MessageBody {
                content: Some(format!("Msg {}", id)),
                content_type: None,
            }),
            from: None,
            created_date_time: Some(time.to_string()),
            reactions: None,
        }
    }

    #[test]
    fn delta_links_initially_empty() {
        let app = App::new();
        assert!(app.chat_delta_links.is_empty());
    }

    #[test]
    fn merge_empty_delta_returns_false() {
        let mut app = App::new();
        app.messages = vec![make_message_with_time("m1", "2026-01-01T10:00:00Z")];
        assert!(!app.merge_delta_messages(Vec::new()));
    }

    #[test]
    fn merge_new_messages_returns_true() {
        let mut app = App::new();
        app.messages = vec![make_message_with_time("m1", "2026-01-01T10:00:00Z")];
        app.known_message_ids.insert("m1".to_string());

        let delta = vec![make_message_with_time("m2", "2026-01-01T10:01:00Z")];
        assert!(app.merge_delta_messages(delta));
        assert_eq!(app.messages.len(), 2);
    }

    #[test]
    fn merge_updates_existing_message() {
        let mut app = App::new();
        app.messages = vec![make_message_with_time("m1", "2026-01-01T10:00:00Z")];
        app.known_message_ids.insert("m1".to_string());

        let mut updated = make_message_with_time("m1", "2026-01-01T10:00:00Z");
        updated.body = Some(MessageBody {
            content: Some("Updated content".to_string()),
            content_type: None,
        });
        let has_new = app.merge_delta_messages(vec![updated]);
        assert!(!has_new); // update, not new
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content_text(), "Updated content");
    }

    #[test]
    fn merge_sorts_by_creation_time() {
        let mut app = App::new();
        app.messages = vec![
            make_message_with_time("m1", "2026-01-01T10:00:00Z"),
            make_message_with_time("m3", "2026-01-01T10:02:00Z"),
        ];
        app.known_message_ids.insert("m1".to_string());
        app.known_message_ids.insert("m3".to_string());

        let delta = vec![make_message_with_time("m2", "2026-01-01T10:01:00Z")];
        app.merge_delta_messages(delta);

        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.messages[0].id, "m1");
        assert_eq!(app.messages[1].id, "m2");
        assert_eq!(app.messages[2].id, "m3");
    }

    #[test]
    fn merge_updates_known_message_ids() {
        let mut app = App::new();
        app.messages = vec![make_message_with_time("m1", "2026-01-01T10:00:00Z")];
        app.known_message_ids.insert("m1".to_string());

        let delta = vec![make_message_with_time("m2", "2026-01-01T10:01:00Z")];
        app.merge_delta_messages(delta);

        assert!(app.known_message_ids.contains("m1"));
        assert!(app.known_message_ids.contains("m2"));
    }

    #[test]
    fn delta_link_can_be_stored_and_retrieved() {
        let mut app = App::new();
        app.chat_delta_links.insert(
            "chat-1".to_string(),
            "https://graph.microsoft.com/delta?token=abc".to_string(),
        );
        assert_eq!(
            app.chat_delta_links.get("chat-1").unwrap(),
            "https://graph.microsoft.com/delta?token=abc"
        );
    }
}

#[cfg(test)]
mod search_tests {
    use ttyms::app::{App, DialogMode, Panel};
    use ttyms::models::{Chat, SearchHit};

    fn make_chat(id: &str) -> Chat {
        Chat {
            id: id.to_string(),
            topic: None,
            chat_type: "oneOnOne".to_string(),
            members: None,
            last_message_preview: None,
            unread_message_count: None,
        }
    }

    #[test]
    fn open_search_sets_dialog() {
        let mut app = App::new();
        app.open_search();
        assert!(matches!(app.dialog, DialogMode::Search));
        assert!(app.search_input.is_empty());
        assert_eq!(app.search_cursor, 0);
        assert!(app.search_results.is_empty());
        assert_eq!(app.selected_search_result, 0);
        assert!(!app.search_loading);
    }

    #[test]
    fn open_search_clears_previous_state() {
        let mut app = App::new();
        app.search_input = "old query".to_string();
        app.search_cursor = 5;
        app.selected_search_result = 3;
        app.open_search();
        assert!(app.search_input.is_empty());
        assert_eq!(app.search_cursor, 0);
        assert_eq!(app.selected_search_result, 0);
    }

    #[test]
    fn navigate_to_known_chat() {
        let mut app = App::new();
        app.chats = vec![make_chat("chat-1"), make_chat("chat-2"), make_chat("chat-3")];
        assert!(app.navigate_to_chat("chat-2"));
        assert_eq!(app.selected_chat, 1);
        assert_eq!(app.active_panel, Panel::Messages);
    }

    #[test]
    fn navigate_to_unknown_chat_returns_false() {
        let mut app = App::new();
        app.chats = vec![make_chat("chat-1")];
        assert!(!app.navigate_to_chat("chat-unknown"));
    }

    #[test]
    fn search_results_navigation() {
        let mut app = App::new();
        let hits: Vec<SearchHit> = (0..5)
            .map(|_| SearchHit {
                summary: Some("test".to_string()),
                resource: None,
            })
            .collect();
        app.search_results = hits;
        app.selected_search_result = 0;

        // Move down
        app.selected_search_result = (app.selected_search_result + 1)
            .min(app.search_results.len().saturating_sub(1));
        assert_eq!(app.selected_search_result, 1);

        // Move to last
        app.selected_search_result = 4;
        app.selected_search_result = (app.selected_search_result + 1)
            .min(app.search_results.len().saturating_sub(1));
        assert_eq!(app.selected_search_result, 4); // Stays at end

        // Move up
        app.selected_search_result = app.selected_search_result.saturating_sub(1);
        assert_eq!(app.selected_search_result, 3);

        // Move to first
        app.selected_search_result = 0;
        app.selected_search_result = app.selected_search_result.saturating_sub(1);
        assert_eq!(app.selected_search_result, 0); // Stays at start
    }

    #[test]
    fn close_dialog_from_search() {
        let mut app = App::new();
        app.open_search();
        app.search_input = "query".to_string();
        app.close_dialog();
        assert!(matches!(app.dialog, DialogMode::None));
    }
}

#[cfg(test)]
mod chat_manager_tests {
    use ttyms::app::{App, ChatManagerTab, DialogMode};
    use ttyms::models::{Chat, ChatMember};

    fn make_chat(id: &str) -> Chat {
        Chat {
            id: id.to_string(),
            topic: Some("Test Group".to_string()),
            chat_type: "group".to_string(),
            members: None,
            last_message_preview: None,
            unread_message_count: None,
        }
    }

    fn make_member(name: &str, user_id: &str, membership_id: &str) -> ChatMember {
        ChatMember {
            id: Some(membership_id.to_string()),
            display_name: Some(name.to_string()),
            user_id: Some(user_id.to_string()),
        }
    }

    #[test]
    fn open_chat_manager_sets_dialog() {
        let mut app = App::new();
        app.open_chat_manager();
        assert!(matches!(app.dialog, DialogMode::ChatManager));
        assert_eq!(app.chat_manager_tab, ChatManagerTab::Members);
        assert!(app.chat_manager_members.is_empty());
        assert!(app.chat_manager_loading);
    }

    #[test]
    fn open_chat_manager_clears_previous_state() {
        let mut app = App::new();
        app.chat_manager_rename_input = "old name".to_string();
        app.chat_manager_rename_cursor = 5;
        app.chat_manager_add_input = "old query".to_string();
        app.open_chat_manager();
        assert!(app.chat_manager_rename_input.is_empty());
        assert_eq!(app.chat_manager_rename_cursor, 0);
        assert!(app.chat_manager_add_input.is_empty());
    }

    #[test]
    fn tab_switching() {
        let mut app = App::new();
        app.open_chat_manager();
        assert_eq!(app.chat_manager_tab, ChatManagerTab::Members);
        app.chat_manager_tab = ChatManagerTab::Rename;
        assert_eq!(app.chat_manager_tab, ChatManagerTab::Rename);
        app.chat_manager_tab = ChatManagerTab::AddMember;
        assert_eq!(app.chat_manager_tab, ChatManagerTab::AddMember);
    }

    #[test]
    fn member_list_navigation() {
        let mut app = App::new();
        app.open_chat_manager();
        app.chat_manager_loading = false;
        app.chat_manager_members = vec![
            make_member("Alice", "u1", "m1"),
            make_member("Bob", "u2", "m2"),
            make_member("Charlie", "u3", "m3"),
        ];
        assert_eq!(app.chat_manager_selected_member, 0);

        app.chat_manager_selected_member = (app.chat_manager_selected_member + 1)
            .min(app.chat_manager_members.len().saturating_sub(1));
        assert_eq!(app.chat_manager_selected_member, 1);

        app.chat_manager_selected_member = (app.chat_manager_selected_member + 1)
            .min(app.chat_manager_members.len().saturating_sub(1));
        assert_eq!(app.chat_manager_selected_member, 2);

        // Stays at end
        app.chat_manager_selected_member = (app.chat_manager_selected_member + 1)
            .min(app.chat_manager_members.len().saturating_sub(1));
        assert_eq!(app.chat_manager_selected_member, 2);

        // Move back up
        app.chat_manager_selected_member = app.chat_manager_selected_member.saturating_sub(1);
        assert_eq!(app.chat_manager_selected_member, 1);
    }

    #[test]
    fn selected_chat_topic_returns_topic() {
        let mut app = App::new();
        app.chats = vec![make_chat("c1")];
        app.selected_chat = 0;
        assert_eq!(app.selected_chat_topic(), "Test Group");
    }

    #[test]
    fn selected_chat_topic_empty_when_none() {
        let mut app = App::new();
        let mut chat = make_chat("c1");
        chat.topic = None;
        app.chats = vec![chat];
        app.selected_chat = 0;
        assert_eq!(app.selected_chat_topic(), "");
    }

    #[test]
    fn selected_chat_is_group() {
        let mut app = App::new();
        app.chats = vec![make_chat("c1")];
        app.selected_chat = 0;
        assert!(app.selected_chat_is_group());
    }

    #[test]
    fn selected_chat_is_not_group() {
        let mut app = App::new();
        let mut chat = make_chat("c1");
        chat.chat_type = "oneOnOne".to_string();
        app.chats = vec![chat];
        app.selected_chat = 0;
        assert!(!app.selected_chat_is_group());
    }

    #[test]
    fn close_chat_manager() {
        let mut app = App::new();
        app.open_chat_manager();
        app.close_dialog();
        assert!(matches!(app.dialog, DialogMode::None));
    }

    #[test]
    fn rename_input_editing() {
        let mut app = App::new();
        app.open_chat_manager();
        app.chat_manager_tab = ChatManagerTab::Rename;

        // Type a name
        for c in "New Name".chars() {
            app.chat_manager_rename_input.insert(app.chat_manager_rename_cursor, c);
            app.chat_manager_rename_cursor += 1;
        }
        assert_eq!(app.chat_manager_rename_input, "New Name");
        assert_eq!(app.chat_manager_rename_cursor, 8);

        // Backspace
        app.chat_manager_rename_cursor -= 1;
        app.chat_manager_rename_input.remove(app.chat_manager_rename_cursor);
        assert_eq!(app.chat_manager_rename_input, "New Nam");
    }
}
