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
