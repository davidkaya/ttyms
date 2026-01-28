use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GraphResponse<T> {
    pub value: Vec<T>,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMember {
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageBody {
    pub content: Option<String>,
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
