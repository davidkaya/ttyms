use anyhow::{Context, Result};
use zeroize::Zeroize;

use crate::models::*;

pub struct GraphClient {
    client: reqwest::Client,
    access_token: String,
}

impl GraphClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.access_token.zeroize();
        self.access_token = token;
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        resp.json::<T>()
            .await
            .context("Failed to parse Graph API response")
    }

    pub async fn get_me(&self) -> Result<User> {
        self.get("https://graph.microsoft.com/v1.0/me").await
    }

    pub async fn list_chats(&self) -> Result<Vec<Chat>> {
        let resp: GraphResponse<Chat> = self
            .get(
                "https://graph.microsoft.com/v1.0/me/chats?\
                 $expand=members,lastMessagePreview&\
                 $orderby=lastMessagePreview/createdDateTime%20desc&\
                 $top=50",
            )
            .await?;
        Ok(resp.value)
    }

    pub async fn get_messages(&self, chat_id: &str) -> Result<Vec<Message>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages?\
             $top=50&$orderby=createdDateTime%20desc",
            chat_id
        );
        let resp: GraphResponse<Message> = self.get(&url).await?;
        let mut messages = resp.value;
        messages.reverse(); // Show oldest first
        Ok(messages)
    }

    pub async fn send_message(&self, chat_id: &str, content: &str) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages",
            chat_id
        );
        let body = serde_json::json!({
            "body": {
                "content": content,
                "contentType": "text"
            }
        });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to send message ({}): {}", status, body);
        }
        resp.json()
            .await
            .context("Failed to parse send response")
    }

    pub async fn create_chat(&self, user_email: &str, my_id: &str) -> Result<Chat> {
        let body = serde_json::json!({
            "chatType": "oneOnOne",
            "members": [
                {
                    "@odata.type": "#microsoft.graph.aadUserConversationMember",
                    "roles": ["owner"],
                    "user@odata.bind": format!("https://graph.microsoft.com/v1.0/users('{}')", my_id)
                },
                {
                    "@odata.type": "#microsoft.graph.aadUserConversationMember",
                    "roles": ["owner"],
                    "user@odata.bind": format!("https://graph.microsoft.com/v1.0/users('{}')", user_email)
                }
            ]
        });
        let resp = self
            .client
            .post("https://graph.microsoft.com/v1.0/chats")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create chat ({}): {}", status, body);
        }
        resp.json().await.context("Failed to parse new chat")
    }

    pub async fn search_users(&self, query: &str) -> Result<Vec<User>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/users?\
             $filter=startswith(displayName,'{}') or startswith(mail,'{}') or startswith(userPrincipalName,'{}')&\
             $top=8&$select=id,displayName,mail,userPrincipalName",
            query, query, query
        );
        let resp: GraphResponse<User> = self.get(&url).await?;
        Ok(resp.value)
    }
}

impl Drop for GraphClient {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
