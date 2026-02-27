//! Tests for the models module: Chat, Message, strip_html, ChannelMember, deserialization

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
                    content_type: None,
                }),
            }),
            unread_message_count: None,
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
                content_type: None,
            }),
            from: sender_name.map(|name| MessageFrom {
                user: Some(MessageUser {
                    display_name: Some(name.to_string()),
                    id: sender_id.map(String::from),
                }),
            }),
            created_date_time: datetime.map(String::from),
            reactions: None,
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
            reactions: None,
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

#[cfg(test)]
mod reaction_tests {
    use ttyms::models::*;

    fn make_message_with_reactions(reactions: Vec<(&str, &str)>) -> Message {
        Message {
            id: "msg-1".to_string(),
            message_type: Some("message".to_string()),
            body: Some(MessageBody {
                content: Some("Hello".to_string()),
                content_type: None,
            }),
            from: None,
            created_date_time: None,
            reactions: Some(
                reactions
                    .into_iter()
                    .map(|(rtype, uid)| ChatMessageReaction {
                        reaction_type: rtype.to_string(),
                        user: Some(ReactionIdentitySet {
                            user: Some(MessageUser {
                                display_name: Some("User".to_string()),
                                id: Some(uid.to_string()),
                            }),
                        }),
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn reactions_summary_counts_by_type() {
        let msg = make_message_with_reactions(vec![
            ("like", "u1"),
            ("like", "u2"),
            ("heart", "u1"),
        ]);
        let summary = msg.reactions_summary();
        assert_eq!(summary.len(), 2);
        // "like" should have count 2, "heart" count 1
        let like_count = summary.iter().find(|(e, _)| e == "👍").map(|(_, c)| *c);
        let heart_count = summary.iter().find(|(e, _)| e == "❤️").map(|(_, c)| *c);
        assert_eq!(like_count, Some(2));
        assert_eq!(heart_count, Some(1));
    }

    #[test]
    fn reactions_summary_empty_when_no_reactions() {
        let msg = Message {
            id: "msg".to_string(),
            message_type: None,
            body: None,
            from: None,
            created_date_time: None,
            reactions: None,
        };
        assert!(msg.reactions_summary().is_empty());
    }

    #[test]
    fn reactions_summary_sorted_by_count_descending() {
        let msg = make_message_with_reactions(vec![
            ("heart", "u1"),
            ("like", "u1"),
            ("like", "u2"),
            ("like", "u3"),
        ]);
        let summary = msg.reactions_summary();
        assert_eq!(summary[0].1, 3); // like: 3
        assert_eq!(summary[1].1, 1); // heart: 1
    }

    #[test]
    fn reaction_emoji_maps_known_types() {
        assert_eq!(reaction_emoji("like"), "👍");
        assert_eq!(reaction_emoji("heart"), "❤️");
        assert_eq!(reaction_emoji("laugh"), "😂");
        assert_eq!(reaction_emoji("surprised"), "😮");
        assert_eq!(reaction_emoji("sad"), "😢");
        assert_eq!(reaction_emoji("angry"), "😡");
    }

    #[test]
    fn reaction_emoji_returns_unknown_type_as_is() {
        assert_eq!(reaction_emoji("custom"), "custom");
    }
}

#[cfg(test)]
mod presence_tests {
    use ttyms::models::*;

    #[test]
    fn presence_indicator_available() {
        let (icon, text) = presence_indicator("Available");
        assert_eq!(icon, "🟢");
        assert_eq!(text, "Available");
    }

    #[test]
    fn presence_indicator_busy() {
        let (icon, _) = presence_indicator("Busy");
        assert_eq!(icon, "🔴");
    }

    #[test]
    fn presence_indicator_dnd() {
        let (icon, _) = presence_indicator("DoNotDisturb");
        assert_eq!(icon, "⛔");
    }

    #[test]
    fn presence_indicator_away() {
        let (icon, _) = presence_indicator("Away");
        assert_eq!(icon, "🟡");
    }

    #[test]
    fn presence_indicator_offline() {
        let (icon, _) = presence_indicator("Offline");
        assert_eq!(icon, "⚫");
    }

    #[test]
    fn presence_indicator_unknown() {
        let (icon, text) = presence_indicator("SomethingElse");
        assert_eq!(icon, "⚪");
        assert_eq!(text, "Unknown");
    }
}

#[cfg(test)]
mod rich_text_tests {
    use ttyms::models::*;

    #[test]
    fn plain_text_returns_plain_segment() {
        let segments = parse_rich_text("Hello world");
        assert_eq!(segments, vec![RichSegment::Plain("Hello world".to_string())]);
    }

    #[test]
    fn bold_tag_returns_bold_segment() {
        let segments = parse_rich_text("before <b>bold</b> after");
        assert!(segments.contains(&RichSegment::Bold("bold".to_string())));
    }

    #[test]
    fn strong_tag_returns_bold_segment() {
        let segments = parse_rich_text("<strong>strong</strong>");
        assert_eq!(segments, vec![RichSegment::Bold("strong".to_string())]);
    }

    #[test]
    fn italic_tag_returns_italic_segment() {
        let segments = parse_rich_text("<i>italic</i>");
        assert_eq!(segments, vec![RichSegment::Italic("italic".to_string())]);
    }

    #[test]
    fn em_tag_returns_italic_segment() {
        let segments = parse_rich_text("<em>emphasis</em>");
        assert_eq!(segments, vec![RichSegment::Italic("emphasis".to_string())]);
    }

    #[test]
    fn code_tag_returns_code_segment() {
        let segments = parse_rich_text("<code>let x = 1</code>");
        assert_eq!(segments, vec![RichSegment::Code("let x = 1".to_string())]);
    }

    #[test]
    fn br_tag_returns_newline() {
        let segments = parse_rich_text("line1<br>line2");
        assert!(segments.contains(&RichSegment::Newline));
    }

    #[test]
    fn link_tag_returns_link_segment() {
        let segments = parse_rich_text(r#"<a href="http://example.com">click</a>"#);
        assert!(segments.contains(&RichSegment::Link {
            text: "click".to_string(),
            url: "http://example.com".to_string(),
        }));
    }

    #[test]
    fn mixed_content_produces_multiple_segments() {
        let segments = parse_rich_text("Hello <b>bold</b> world");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], RichSegment::Plain("Hello ".to_string()));
        assert_eq!(segments[1], RichSegment::Bold("bold".to_string()));
        assert_eq!(segments[2], RichSegment::Plain(" world".to_string()));
    }

    #[test]
    fn entities_decoded_in_rich_text() {
        let segments = parse_rich_text("A &amp; B");
        assert_eq!(segments, vec![RichSegment::Plain("A & B".to_string())]);
    }

    #[test]
    fn empty_input_returns_plain_empty() {
        let segments = parse_rich_text("");
        assert_eq!(segments.len(), 1);
    }
}

#[cfg(test)]
mod unread_tests {
    use ttyms::models::*;

    #[test]
    fn unread_count_returns_value() {
        let chat = Chat {
            id: "c1".to_string(),
            topic: None,
            chat_type: "oneOnOne".to_string(),
            members: None,
            last_message_preview: None,
            unread_message_count: Some(5),
        };
        assert_eq!(chat.unread_count(), 5);
    }

    #[test]
    fn unread_count_defaults_to_zero() {
        let chat = Chat {
            id: "c1".to_string(),
            topic: None,
            chat_type: "oneOnOne".to_string(),
            members: None,
            last_message_preview: None,
            unread_message_count: None,
        };
        assert_eq!(chat.unread_count(), 0);
    }

    #[test]
    fn deserialize_chat_with_unread() {
        let json = r#"{
            "id": "chat1",
            "chatType": "oneOnOne",
            "unreadMessageCount": 3
        }"#;
        let chat: Chat = serde_json::from_str(json).unwrap();
        assert_eq!(chat.unread_count(), 3);
    }

    #[test]
    fn deserialize_message_with_reactions() {
        let json = r#"{
            "id": "msg1",
            "messageType": "message",
            "body": {"content": "Hello"},
            "reactions": [
                {"reactionType": "like", "user": {"user": {"id": "u1", "displayName": "Alice"}}},
                {"reactionType": "like", "user": {"user": {"id": "u2", "displayName": "Bob"}}}
            ]
        }"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.reactions.as_ref().unwrap().len(), 2);
        let summary = msg.reactions_summary();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0], ("👍".to_string(), 2));
    }
}

#[cfg(test)]
mod team_channel_deserialization {
    use ttyms::models::*;

    #[test]
    fn deserialize_team() {
        let json = r#"{"id":"t1","displayName":"Engineering","description":"Dev team"}"#;
        let team: Team = serde_json::from_str(json).unwrap();
        assert_eq!(team.id, "t1");
        assert_eq!(team.display_name, "Engineering");
        assert_eq!(team.description.as_deref(), Some("Dev team"));
    }

    #[test]
    fn deserialize_channel() {
        let json = r#"{"id":"c1","displayName":"General","membershipType":"standard"}"#;
        let channel: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(channel.id, "c1");
        assert_eq!(channel.display_name, "General");
        assert_eq!(channel.membership_type.as_deref(), Some("standard"));
    }

    #[test]
    fn deserialize_channel_private() {
        let json = r#"{"id":"c2","displayName":"Secret","membershipType":"private"}"#;
        let channel: Channel = serde_json::from_str(json).unwrap();
        assert_eq!(channel.membership_type.as_deref(), Some("private"));
    }

    #[test]
    fn deserialize_presence() {
        let json = r#"{"availability":"Available","activity":"Available"}"#;
        let presence: Presence = serde_json::from_str(json).unwrap();
        assert_eq!(presence.availability.as_deref(), Some("Available"));
    }

    #[test]
    fn deserialize_teams_response() {
        let json = r#"{"value":[{"id":"t1","displayName":"Team A"},{"id":"t2","displayName":"Team B"}]}"#;
        let resp: GraphResponse<Team> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 2);
    }
}

#[cfg(test)]
mod paged_response_tests {
    use ttyms::models::*;

    #[test]
    fn deserialize_paged_response_with_next_link() {
        let json = r#"{
            "value": [{"id":"msg1","messageType":"message"}],
            "@odata.nextLink": "https://graph.microsoft.com/v1.0/me/chats/abc/messages?$skiptoken=xyz"
        }"#;
        let resp: PagedResponse<Message> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 1);
        assert_eq!(
            resp.next_link.as_deref(),
            Some("https://graph.microsoft.com/v1.0/me/chats/abc/messages?$skiptoken=xyz")
        );
    }

    #[test]
    fn deserialize_paged_response_without_next_link() {
        let json = r#"{"value": [{"id":"msg1","messageType":"message"}]}"#;
        let resp: PagedResponse<Message> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 1);
        assert!(resp.next_link.is_none());
    }

    #[test]
    fn deserialize_paged_response_empty() {
        let json = r#"{"value": []}"#;
        let resp: PagedResponse<Message> = serde_json::from_str(json).unwrap();
        assert!(resp.value.is_empty());
        assert!(resp.next_link.is_none());
    }
}

#[cfg(test)]
mod channel_member_tests {
    use ttyms::models::ChannelMember;

    fn make_member(name: Option<&str>, roles: Vec<&str>) -> ChannelMember {
        ChannelMember {
            id: Some("u1".to_string()),
            display_name: name.map(|n| n.to_string()),
            roles: roles.into_iter().map(|r| r.to_string()).collect(),
        }
    }

    #[test]
    fn name_returns_display_name() {
        let m = make_member(Some("Alice"), vec![]);
        assert_eq!(m.name(), "Alice");
    }

    #[test]
    fn name_returns_unknown_when_none() {
        let m = make_member(None, vec![]);
        assert_eq!(m.name(), "Unknown");
    }

    #[test]
    fn is_owner_true_when_role_present() {
        let m = make_member(Some("Alice"), vec!["owner"]);
        assert!(m.is_owner());
    }

    #[test]
    fn is_owner_false_when_no_roles() {
        let m = make_member(Some("Bob"), vec![]);
        assert!(!m.is_owner());
    }

    #[test]
    fn is_owner_false_for_other_roles() {
        let m = make_member(Some("Charlie"), vec!["guest"]);
        assert!(!m.is_owner());
    }

    #[test]
    fn deserialize_channel_member() {
        let json = r#"{
            "id": "u1",
            "displayName": "Alice Smith",
            "roles": ["owner"]
        }"#;
        let m: ChannelMember = serde_json::from_str(json).unwrap();
        assert_eq!(m.name(), "Alice Smith");
        assert!(m.is_owner());
    }

    #[test]
    fn deserialize_channel_member_minimal() {
        let json = r#"{"roles": []}"#;
        let m: ChannelMember = serde_json::from_str(json).unwrap();
        assert_eq!(m.name(), "Unknown");
        assert!(!m.is_owner());
        assert!(m.id.is_none());
    }

    #[test]
    fn deserialize_channel_member_default_roles() {
        let json = r#"{"id": "u1", "displayName": "Bob"}"#;
        let m: ChannelMember = serde_json::from_str(json).unwrap();
        assert!(!m.is_owner());
        assert!(m.roles.is_empty());
    }
}

#[cfg(test)]
mod delta_response_deserialization {
    use ttyms::models::{DeltaResponse, Message};

    #[test]
    fn deserialize_delta_with_delta_link() {
        let json = r#"{
            "value": [
                {
                    "id": "msg-1",
                    "messageType": "message",
                    "createdDateTime": "2026-01-01T10:00:00Z"
                }
            ],
            "@odata.deltaLink": "https://graph.microsoft.com/v1.0/chats/c1/messages/delta?$deltatoken=abc123"
        }"#;
        let resp: DeltaResponse<Message> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 1);
        assert_eq!(resp.value[0].id, "msg-1");
        assert!(resp.next_link.is_none());
        assert_eq!(
            resp.delta_link.unwrap(),
            "https://graph.microsoft.com/v1.0/chats/c1/messages/delta?$deltatoken=abc123"
        );
    }

    #[test]
    fn deserialize_delta_with_next_link() {
        let json = r#"{
            "value": [
                {"id": "msg-1", "messageType": "message"},
                {"id": "msg-2", "messageType": "message"}
            ],
            "@odata.nextLink": "https://graph.microsoft.com/v1.0/chats/c1/messages/delta?$skiptoken=xyz"
        }"#;
        let resp: DeltaResponse<Message> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 2);
        assert!(resp.delta_link.is_none());
        assert_eq!(
            resp.next_link.unwrap(),
            "https://graph.microsoft.com/v1.0/chats/c1/messages/delta?$skiptoken=xyz"
        );
    }

    #[test]
    fn deserialize_empty_delta_response() {
        let json = r#"{
            "value": [],
            "@odata.deltaLink": "https://graph.microsoft.com/v1.0/chats/c1/messages/delta?$deltatoken=empty"
        }"#;
        let resp: DeltaResponse<Message> = serde_json::from_str(json).unwrap();
        assert!(resp.value.is_empty());
        assert!(resp.delta_link.is_some());
    }
}

#[cfg(test)]
mod search_response_tests {
    use ttyms::models::{SearchHit, SearchResponse};

    #[test]
    fn deserialize_search_response_with_hits() {
        let json = r#"{
            "value": [{
                "hitsContainers": [{
                    "hits": [
                        {
                            "summary": "Hello <b>world</b>",
                            "resource": {
                                "id": "msg-1",
                                "createdDateTime": "2026-01-15T10:30:00Z",
                                "chatId": "chat-123",
                                "from": {
                                    "emailAddress": {
                                        "name": "Alice Smith",
                                        "address": "alice@example.com"
                                    }
                                }
                            }
                        }
                    ],
                    "total": 1,
                    "moreResultsAvailable": false
                }]
            }]
        }"#;
        let resp: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value.len(), 1);
        let hits = &resp.value[0].hits_containers[0].hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sender_name(), "Alice Smith");
        assert_eq!(hits[0].chat_id(), Some("chat-123"));
        assert_eq!(hits[0].summary_text(), "Hello world");
    }

    #[test]
    fn deserialize_search_response_empty() {
        let json = r#"{
            "value": [{
                "hitsContainers": [{
                    "hits": [],
                    "total": 0,
                    "moreResultsAvailable": false
                }]
            }]
        }"#;
        let resp: SearchResponse = serde_json::from_str(json).unwrap();
        let hits = &resp.value[0].hits_containers[0].hits;
        assert!(hits.is_empty());
    }

    #[test]
    fn search_hit_sender_name_fallback() {
        let hit = SearchHit {
            summary: Some("test".to_string()),
            resource: None,
        };
        assert_eq!(hit.sender_name(), "Unknown");
    }

    #[test]
    fn search_hit_summary_strips_html() {
        let json = r#"{
            "summary": "<b>important</b> meeting <em>tomorrow</em>",
            "resource": null
        }"#;
        let hit: SearchHit = serde_json::from_str(json).unwrap();
        assert_eq!(hit.summary_text(), "important meeting tomorrow");
    }

    #[test]
    fn search_hit_formatted_time() {
        let json = r#"{
            "summary": "test",
            "resource": {
                "createdDateTime": "2026-01-15T10:30:00Z",
                "chatId": "chat-1"
            }
        }"#;
        let hit: SearchHit = serde_json::from_str(json).unwrap();
        // Should produce a non-empty formatted time string
        assert!(!hit.formatted_time().is_empty());
    }

    #[test]
    fn search_hit_empty_summary() {
        let hit = SearchHit {
            summary: None,
            resource: None,
        };
        assert_eq!(hit.summary_text(), "");
        assert_eq!(hit.formatted_time(), "");
    }
}
