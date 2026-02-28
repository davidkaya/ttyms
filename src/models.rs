use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct GraphResponse<T> {
    pub value: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PagedResponse<T> {
    pub value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeltaResponse<T> {
    pub value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    pub delta_link: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "mail", default)]
    pub mail: Option<String>,
    #[serde(rename = "userPrincipalName", default)]
    pub user_principal_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Chat {
    pub id: String,
    pub topic: Option<String>,
    #[serde(rename = "chatType")]
    pub chat_type: String,
    pub members: Option<Vec<ChatMember>>,
    #[serde(rename = "lastMessagePreview")]
    pub last_message_preview: Option<MessagePreview>,
    #[serde(rename = "unreadMessageCount", default)]
    pub unread_message_count: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMember {
    pub id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePreview {
    pub body: Option<MessageBody>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Message {
    pub id: String,
    #[serde(rename = "messageType")]
    pub message_type: Option<String>,
    pub body: Option<MessageBody>,
    pub from: Option<MessageFrom>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: Option<String>,
    #[serde(default)]
    pub reactions: Option<Vec<ChatMessageReaction>>,
    #[serde(default)]
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MessageBody {
    pub content: Option<String>,
    #[serde(rename = "contentType", default)]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageFrom {
    pub user: Option<MessageUser>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageUser {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChatMessageAttachment {
    pub id: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    #[serde(rename = "contentUrl")]
    pub content_url: Option<String>,
    pub name: Option<String>,
}

// Reaction types
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChatMessageReaction {
    #[serde(rename = "reactionType")]
    pub reaction_type: String,
    pub user: Option<ReactionIdentitySet>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ReactionIdentitySet {
    pub user: Option<MessageUser>,
}

// Teams & Channels
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Team {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Channel {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
    #[serde(rename = "membershipType", default)]
    pub membership_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChannelMember {
    pub id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl ChannelMember {
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or("Unknown")
    }

    pub fn is_owner(&self) -> bool {
        self.roles.iter().any(|r| r == "owner")
    }
}

// Presence
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Presence {
    pub availability: Option<String>,
    pub activity: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresenceResponse {
    pub value: Vec<UserPresence>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserPresence {
    pub id: String,
    pub availability: Option<String>,
}

impl Chat {
    pub fn display_name(&self, current_user_id: &str) -> String {
        if let Some(ref topic) = self.topic {
            if !topic.is_empty() {
                return topic.clone();
            }
        }
        if let Some(ref members) = self.members {
            let others: Vec<&str> = members
                .iter()
                .filter(|m| m.user_id.as_deref() != Some(current_user_id))
                .filter_map(|m| m.display_name.as_deref())
                .collect();
            if !others.is_empty() {
                return others.join(", ");
            }
        }
        "Chat".to_string()
    }

    pub fn preview_text(&self) -> String {
        self.last_message_preview
            .as_ref()
            .and_then(|p| p.body.as_ref())
            .and_then(|b| b.content.as_ref())
            .map(|c| strip_html(c))
            .unwrap_or_default()
    }

    pub fn unread_count(&self) -> i32 {
        self.unread_message_count.unwrap_or(0)
    }
}

impl Message {
    pub fn sender_name(&self) -> String {
        self.from
            .as_ref()
            .and_then(|f| f.user.as_ref())
            .and_then(|u| u.display_name.clone())
            .unwrap_or_else(|| "System".to_string())
    }

    pub fn content_text(&self) -> String {
        self.body
            .as_ref()
            .and_then(|b| b.content.as_ref())
            .map(|c| strip_html(c))
            .unwrap_or_default()
    }

    pub fn sender_id(&self) -> Option<&str> {
        self.from
            .as_ref()
            .and_then(|f| f.user.as_ref())
            .and_then(|u| u.id.as_deref())
    }

    pub fn formatted_time(&self) -> String {
        self.created_date_time
            .as_ref()
            .and_then(|dt| chrono::DateTime::parse_from_rfc3339(dt).ok())
            .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
            .unwrap_or_default()
    }

    pub fn is_user_message(&self) -> bool {
        self.message_type.as_deref() == Some("message")
    }

    /// Returns a summary of reactions as (emoji, count) pairs
    pub fn reactions_summary(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        if let Some(ref reactions) = self.reactions {
            for r in reactions {
                *counts.entry(r.reaction_type.clone()).or_insert(0) += 1;
            }
        }
        let mut result: Vec<_> = counts
            .into_iter()
            .map(|(rtype, count)| (reaction_emoji(&rtype), count))
            .collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    pub fn file_attachments(&self) -> Vec<&ChatMessageAttachment> {
        self.attachments
            .iter()
            .filter(|a| a.content_type.as_deref() == Some("reference"))
            .collect()
    }
}

pub fn strip_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

pub fn reaction_emoji(reaction_type: &str) -> String {
    match reaction_type {
        "like" => "👍".to_string(),
        "heart" => "❤️".to_string(),
        "laugh" => "😂".to_string(),
        "surprised" => "😮".to_string(),
        "sad" => "😢".to_string(),
        "angry" => "😡".to_string(),
        other => other.to_string(),
    }
}

pub fn presence_indicator(availability: &str) -> (&str, &str) {
    match availability {
        "Available" => ("🟢", "Available"),
        "Busy" => ("🔴", "Busy"),
        "DoNotDisturb" => ("⛔", "Do Not Disturb"),
        "Away" => ("🟡", "Away"),
        "BeRightBack" => ("🟡", "Be Right Back"),
        "Offline" => ("⚫", "Offline"),
        "PresenceUnknown" => ("⚪", "Unknown"),
        _ => ("⚪", "Unknown"),
    }
}

/// Available reaction types for the reaction picker
/// Reaction types: (emoji_to_send, display_label)
/// The Graph API setReaction expects unicode emoji as reactionType
pub const REACTION_TYPES: &[(&str, &str)] = &[
    ("👍", "👍 Like"),
    ("❤️", "❤️ Heart"),
    ("😂", "😂 Laugh"),
    ("😮", "😮 Surprised"),
    ("😢", "😢 Sad"),
    ("😡", "😡 Angry"),
];

/// Available presence statuses
pub const PRESENCE_STATUSES: &[(&str, &str)] = &[
    ("Available", "🟢 Available"),
    ("Busy", "🔴 Busy"),
    ("DoNotDisturb", "⛔ Do Not Disturb"),
    ("Away", "🟡 Away"),
    ("BeRightBack", "🟡 Be Right Back"),
    ("Offline", "⚫ Appear Offline"),
];

// ---- DriveItem (file upload response) ----

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DriveItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "webUrl")]
    pub web_url: String,
    #[serde(rename = "eTag", default)]
    pub e_tag: Option<String>,
    #[serde(default)]
    pub size: Option<i64>,
}

// ---- Search results ----

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub value: Vec<SearchResultSet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResultSet {
    #[serde(rename = "hitsContainers", default)]
    pub hits_containers: Vec<HitsContainer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HitsContainer {
    #[serde(default)]
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    #[allow(dead_code)]
    pub total: i32,
    #[serde(rename = "moreResultsAvailable", default)]
    #[allow(dead_code)]
    pub more_results_available: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchHit {
    #[serde(default)]
    pub summary: Option<String>,
    pub resource: Option<SearchChatMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchChatMessage {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: Option<String>,
    pub from: Option<SearchFrom>,
    #[serde(rename = "chatId")]
    pub chat_id: Option<String>,
    #[serde(rename = "channelIdentity")]
    #[allow(dead_code)]
    pub channel_identity: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchFrom {
    #[serde(rename = "emailAddress")]
    pub email_address: Option<SearchEmailAddress>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchEmailAddress {
    pub name: Option<String>,
    #[allow(dead_code)]
    pub address: Option<String>,
}

impl SearchHit {
    pub fn sender_name(&self) -> String {
        self.resource
            .as_ref()
            .and_then(|r| r.from.as_ref())
            .and_then(|f| f.email_address.as_ref())
            .and_then(|e| e.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn summary_text(&self) -> String {
        self.summary
            .as_ref()
            .map(|s| strip_html(s))
            .unwrap_or_default()
    }

    pub fn chat_id(&self) -> Option<&str> {
        self.resource
            .as_ref()
            .and_then(|r| r.chat_id.as_deref())
    }

    pub fn formatted_time(&self) -> String {
        self.resource
            .as_ref()
            .and_then(|r| r.created_date_time.as_ref())
            .and_then(|dt| chrono::DateTime::parse_from_rfc3339(dt).ok())
            .map(|dt| dt.with_timezone(&chrono::Local).format("%m/%d %H:%M").to_string())
            .unwrap_or_default()
    }
}

/// Rich text segment for terminal rendering
#[derive(Debug, Clone, PartialEq)]
pub enum RichSegment {
    Plain(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, url: String },
    Newline,
}

/// Parse HTML into rich text segments for terminal display
pub fn parse_rich_text(html: &str) -> Vec<RichSegment> {
    let mut segments: Vec<RichSegment> = Vec::new();
    let mut chars = html.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Collect tag
            let mut tag = String::new();
            while let Some(&tc) = chars.peek() {
                chars.next();
                if tc == '>' {
                    break;
                }
                tag.push(tc);
            }
            let tag_lower = tag.to_lowercase();

            if tag_lower == "br" || tag_lower == "br/" || tag_lower == "br /" {
                if !current.is_empty() {
                    segments.push(RichSegment::Plain(decode_entities(&current)));
                    current.clear();
                }
                segments.push(RichSegment::Newline);
            } else if tag_lower == "b" || tag_lower == "strong" {
                if !current.is_empty() {
                    segments.push(RichSegment::Plain(decode_entities(&current)));
                    current.clear();
                }
                let inner = collect_until_close(&mut chars, &tag_lower);
                if !inner.is_empty() {
                    segments.push(RichSegment::Bold(decode_entities(&strip_html(&inner))));
                }
            } else if tag_lower == "i" || tag_lower == "em" {
                if !current.is_empty() {
                    segments.push(RichSegment::Plain(decode_entities(&current)));
                    current.clear();
                }
                let inner = collect_until_close(&mut chars, &tag_lower);
                if !inner.is_empty() {
                    segments.push(RichSegment::Italic(decode_entities(&strip_html(&inner))));
                }
            } else if tag_lower == "code" || tag_lower == "pre" {
                if !current.is_empty() {
                    segments.push(RichSegment::Plain(decode_entities(&current)));
                    current.clear();
                }
                let inner = collect_until_close(&mut chars, &tag_lower);
                if !inner.is_empty() {
                    segments.push(RichSegment::Code(decode_entities(&strip_html(&inner))));
                }
            } else if tag_lower.starts_with("a ") {
                if !current.is_empty() {
                    segments.push(RichSegment::Plain(decode_entities(&current)));
                    current.clear();
                }
                let url = extract_href(&tag);
                let inner = collect_until_close(&mut chars, "a");
                let text = decode_entities(&strip_html(&inner));
                if !text.is_empty() {
                    segments.push(RichSegment::Link { text, url });
                }
            }
            // Skip closing tags and other tags
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        segments.push(RichSegment::Plain(decode_entities(&current)));
    }

    // If we only got plain segments, simplify
    if segments.is_empty() {
        segments.push(RichSegment::Plain(String::new()));
    }

    segments
}

fn collect_until_close(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    tag_name: &str,
) -> String {
    let mut result = String::new();
    let close = format!("/{}", tag_name);
    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut inner_tag = String::new();
            while let Some(&tc) = chars.peek() {
                chars.next();
                if tc == '>' {
                    break;
                }
                inner_tag.push(tc);
            }
            if inner_tag.to_lowercase() == close {
                break;
            }
            // Preserve inner tags for nested handling
            result.push('<');
            result.push_str(&inner_tag);
            result.push('>');
        } else {
            result.push(ch);
        }
    }
    result
}

fn extract_href(tag: &str) -> String {
    if let Some(pos) = tag.find("href=\"") {
        let start = pos + 6;
        if let Some(end) = tag[start..].find('"') {
            return tag[start..start + end].to_string();
        }
    }
    if let Some(pos) = tag.find("href='") {
        let start = pos + 6;
        if let Some(end) = tag[start..].find('\'') {
            return tag[start..start + end].to_string();
        }
    }
    String::new()
}

fn decode_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
}
