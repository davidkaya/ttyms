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

    /// Create an independent copy for background tasks (token is cloned, zeroized on drop)
    pub fn clone_for_background(&self) -> Self {
        Self {
            client: self.client.clone(), // cheap: reqwest::Client is Arc internally
            access_token: self.access_token.clone(),
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

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
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

    async fn post_no_content(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        Ok(())
    }

    // ---- User & Profile ----

    pub async fn get_me(&self) -> Result<User> {
        self.get("https://graph.microsoft.com/v1.0/me").await
    }

    // ---- Chats ----

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
        self.post_json(&url, &body).await
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
        self.post_json("https://graph.microsoft.com/v1.0/chats", &body)
            .await
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

    // ---- Reactions ----

    pub async fn set_reaction(
        &self,
        chat_id: &str,
        message_id: &str,
        reaction_type: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages/{}/setReaction",
            chat_id, message_id
        );
        let body = serde_json::json!({ "reactionType": reaction_type });
        self.post_no_content(&url, &body).await
    }

    #[allow(dead_code)]
    pub async fn unset_reaction(
        &self,
        chat_id: &str,
        message_id: &str,
        reaction_type: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages/{}/unsetReaction",
            chat_id, message_id
        );
        let body = serde_json::json!({ "reactionType": reaction_type });
        self.post_no_content(&url, &body).await
    }

    // ---- Presence ----

    pub async fn get_my_presence(&self) -> Result<Presence> {
        self.get("https://graph.microsoft.com/v1.0/me/presence")
            .await
    }

    pub async fn get_presences(&self, user_ids: &[String]) -> Result<Vec<UserPresence>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let body = serde_json::json!({ "ids": user_ids });
        let resp: PresenceResponse = self
            .post_json(
                "https://graph.microsoft.com/v1.0/communications/getPresencesByUserId",
                &body,
            )
            .await?;
        Ok(resp.value)
    }

    pub async fn set_my_presence(
        &self,
        availability: &str,
        activity: &str,
    ) -> Result<()> {
        let url = "https://graph.microsoft.com/v1.0/me/presence/setUserPreferredPresence";
        let body = serde_json::json!({
            "availability": availability,
            "activity": activity,
            "expirationDuration": "PT8H"
        });
        self.post_no_content(url, &body).await
    }

    // ---- Teams ----

    pub async fn list_teams(&self) -> Result<Vec<Team>> {
        let resp: GraphResponse<Team> = self
            .get("https://graph.microsoft.com/v1.0/me/joinedTeams")
            .await?;
        Ok(resp.value)
    }

    // ---- Channels ----

    pub async fn list_channels(&self, team_id: &str) -> Result<Vec<Channel>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels",
            team_id
        );
        let resp: GraphResponse<Channel> = self.get(&url).await?;
        Ok(resp.value)
    }

    pub async fn get_channel_messages(
        &self,
        team_id: &str,
        channel_id: &str,
    ) -> Result<Vec<Message>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages?$top=50",
            team_id, channel_id
        );
        let resp: GraphResponse<Message> = self.get(&url).await?;
        let mut messages = resp.value;
        messages.reverse();
        Ok(messages)
    }

    pub async fn send_channel_message(
        &self,
        team_id: &str,
        channel_id: &str,
        content: &str,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages",
            team_id, channel_id
        );
        let body = serde_json::json!({
            "body": {
                "content": content,
                "contentType": "text"
            }
        });
        self.post_json(&url, &body).await
    }

    #[allow(dead_code)]
    pub async fn reply_to_channel_message(
        &self,
        team_id: &str,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages/{}/replies",
            team_id, channel_id, message_id
        );
        let body = serde_json::json!({
            "body": {
                "content": content,
                "contentType": "text"
            }
        });
        self.post_json(&url, &body).await
    }

    // ---- Read receipts ----

    pub async fn mark_chat_read(&self, chat_id: &str, user_id: &str) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/markChatReadForUser",
            chat_id
        );
        let body = serde_json::json!({
            "user": {
                "id": user_id,
                "tenantId": serde_json::Value::Null
            }
        });
        // This endpoint sometimes returns 204 No Content
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 204 {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to mark chat read ({}): {}", status, body);
        }
        Ok(())
    }
}

impl Drop for GraphClient {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
