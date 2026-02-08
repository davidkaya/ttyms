//! Tests for the models module: Chat, Message, strip_html, deserialization

// We test via integration tests accessing the public API of the crate.
// For unit tests inline in modules, see #[cfg(test)] blocks in source files.

#[cfg(test)]
mod strip_html_tests {
    use ttyms::models::strip_html;

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(strip_html("Hello world"), "Hello world");
    }

    #[test]
    fn removes_simple_tags() {
        assert_eq!(strip_html("<p>Hello</p>"), "Hello");
    }

    #[test]
    fn removes_nested_tags() {
        assert_eq!(
            strip_html("<div><b>Bold</b> and <i>italic</i></div>"),
            "Bold and italic"
        );
    }

    #[test]
    fn decodes_html_entities() {
        assert_eq!(strip_html("A &amp; B &lt; C &gt; D"), "A & B < C > D");
        assert_eq!(strip_html("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(strip_html("it&#39;s"), "it's");
        assert_eq!(strip_html("non&nbsp;breaking"), "non breaking");
    }

    #[test]
    fn handles_empty_string() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn handles_only_tags() {
        assert_eq!(strip_html("<br/><hr/>"), "");
    }

    #[test]
    fn preserves_text_between_tags() {
        assert_eq!(
            strip_html("<p>First</p><p>Second</p>"),
            "FirstSecond"
        );
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(strip_html("  hello  "), "hello");
        assert_eq!(strip_html("<p>  spaced  </p>"), "spaced");
    }

    #[test]
    fn handles_unclosed_tags() {
        assert_eq!(strip_html("<p>unclosed"), "unclosed");
    }

    #[test]
    fn handles_tag_with_attributes() {
        assert_eq!(
            strip_html(r#"<a href="http://example.com">link</a>"#),
            "link"
        );
    }
}

#[cfg(test)]
mod chat_tests {
    use ttyms::models::*;

    fn make_chat(
        topic: Option<&str>,
        members: Option<Vec<(&str, &str)>>,
        preview: Option<&str>,
    ) -> Chat {
        Chat {
            id: "chat-1".to_string(),
            topic: topic.map(String::from),
            chat_type: "oneOnOne".to_string(),
            members: members.map(|m| {
                m.into_iter()
                    .map(|(name, id)| ChatMember {
                        display_name: Some(name.to_string()),
                        user_id: Some(id.to_string()),
                    })
                    .collect()
            }),
            last_message_preview: preview.map(|p| MessagePreview {
                body: Some(MessageBody {
                    content: Some(p.to_string()),
                }),
            }),
        }
    }

    #[test]
    fn display_name_uses_topic_when_present() {
        let chat = make_chat(Some("Project Discussion"), None, None);
        assert_eq!(chat.display_name("me"), "Project Discussion");
    }

    #[test]
    fn display_name_uses_members_when_no_topic() {
        let chat = make_chat(
            None,
            Some(vec![("Me", "me"), ("Alice", "alice"), ("Bob", "bob")]),
            None,
        );
        assert_eq!(chat.display_name("me"), "Alice, Bob");
    }

    #[test]
    fn display_name_skips_empty_topic() {
        let chat = make_chat(
            Some(""),
            Some(vec![("Me", "me"), ("Charlie", "charlie")]),
            None,
        );
        assert_eq!(chat.display_name("me"), "Charlie");
    }

    #[test]
    fn display_name_fallback_when_no_topic_or_members() {
        let chat = make_chat(None, None, None);
        assert_eq!(chat.display_name("me"), "Chat");
    }

    #[test]
    fn preview_text_strips_html() {
        let chat = make_chat(None, None, Some("<p>Hello <b>world</b></p>"));
        assert_eq!(chat.preview_text(), "Hello world");
    }

    #[test]
    fn preview_text_empty_when_no_preview() {
        let chat = make_chat(None, None, None);
        assert_eq!(chat.preview_text(), "");
    }
}

#[cfg(test)]
mod message_tests {
    use ttyms::models::*;

    fn make_message(
        sender_name: Option<&str>,
        sender_id: Option<&str>,
        content: Option<&str>,
        message_type: Option<&str>,
        datetime: Option<&str>,
    ) -> Message {
        Message {
            id: "msg-1".to_string(),
            message_type: message_type.map(String::from),
            body: content.map(|c| MessageBody {
                content: Some(c.to_string()),
            }),
            from: sender_name.map(|name| MessageFrom {
                user: Some(MessageUser {
                    display_name: Some(name.to_string()),
                    id: sender_id.map(String::from),
                }),
            }),
            created_date_time: datetime.map(String::from),
        }
    }

    #[test]
    fn sender_name_returns_display_name() {
        let msg = make_message(Some("Alice"), None, None, None, None);
        assert_eq!(msg.sender_name(), "Alice");
    }

    #[test]
    fn sender_name_returns_system_when_no_from() {
        let msg = make_message(None, None, None, None, None);
        assert_eq!(msg.sender_name(), "System");
    }

    #[test]
    fn content_text_strips_html() {
        let msg = make_message(None, None, Some("<p>Hello</p>"), None, None);
        assert_eq!(msg.content_text(), "Hello");
    }

    #[test]
    fn content_text_empty_when_no_body() {
        let msg = Message {
            id: "msg".to_string(),
            message_type: None,
            body: None,
            from: None,
            created_date_time: None,
        };
        assert_eq!(msg.content_text(), "");
    }

    #[test]
    fn sender_id_returns_some() {
        let msg = make_message(Some("A"), Some("user-123"), None, None, None);
        assert_eq!(msg.sender_id(), Some("user-123"));
    }

    #[test]
    fn sender_id_returns_none_when_no_from() {
        let msg = make_message(None, None, None, None, None);
        assert_eq!(msg.sender_id(), None);
    }

    #[test]
    fn is_user_message_true() {
        let msg = make_message(None, None, None, Some("message"), None);
        assert!(msg.is_user_message());
    }

    #[test]
    fn is_user_message_false_for_system() {
        let msg = make_message(None, None, None, Some("systemEventMessage"), None);
        assert!(!msg.is_user_message());
    }

    #[test]
    fn is_user_message_false_when_none() {
        let msg = make_message(None, None, None, None, None);
        assert!(!msg.is_user_message());
    }

    #[test]
    fn formatted_time_parses_rfc3339() {
        let msg = make_message(None, None, None, None, Some("2026-02-15T10:30:00Z"));
        let time = msg.formatted_time();
        // We can't assert exact time due to timezone, but it should be non-empty
        assert!(!time.is_empty(), "formatted_time should produce a time string");
        assert!(
            time.contains(':'),
            "formatted_time should contain a colon: got '{}'",
            time
        );
    }

    #[test]
    fn formatted_time_empty_when_none() {
        let msg = make_message(None, None, None, None, None);
        assert_eq!(msg.formatted_time(), "");
    }

    #[test]
    fn formatted_time_empty_for_invalid_datetime() {
        let msg = make_message(None, None, None, None, Some("not-a-date"));
        assert_eq!(msg.formatted_time(), "");
    }
}

#[cfg(test)]
mod deserialization_tests {
    use ttyms::models::*;

    #[test]
    fn deserialize_user() {
        let json = r#"{"id":"123","displayName":"Test User","mail":"test@example.com"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, "123");
        assert_eq!(user.display_name, "Test User");
        assert_eq!(user.mail.as_deref(), Some("test@example.com"));
    }

    #[test]
    fn deserialize_user_without_optional_fields() {
        let json = r#"{"id":"123","displayName":"Test User"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, "123");
        assert!(user.mail.is_none());
        assert!(user.user_principal_name.is_none());
    }

    #[test]
    fn deserialize_message() {
        let json = r#"{
            "id": "msg1",
            "messageType": "message",
            "body": {"content": "Hello"},
            "from": {"user": {"displayName": "Alice", "id": "u1"}},
            "createdDateTime": "2026-02-15T10:00:00Z"
        }"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "msg1");
        assert_eq!(msg.content_text(), "Hello");
        assert_eq!(msg.sender_name(), "Alice");
        assert!(msg.is_user_message());
    }

    #[test]
    fn deserialize_chat() {
        let json = r#"{
            "id": "chat1",
            "chatType": "oneOnOne",
            "topic": "Team Chat"
        }"#;
        let chat: Chat = serde_json::from_str(json).unwrap();
        assert_eq!(chat.id, "chat1");
        assert_eq!(chat.topic.as_deref(), Some("Team Chat"));
        assert!(chat.members.is_none());
    }

    #[test]
    fn deserialize_graph_response() {
        let json = r#"{"value": [{"id":"1","displayName":"A"},{"id":"2","displayName":"B"}]}"#;
        let resp: GraphResponse<User> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 2);
        assert_eq!(resp.value[0].display_name, "A");
    }
}
