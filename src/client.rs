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

    async fn patch_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let resp = self
            .client
            .patch(url)
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

    async fn post_no_response(&self, url: &str) -> Result<()> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Length", "0")
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        Ok(())
    }

    async fn patch_no_content(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let resp = self
            .client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, text);
        }
        Ok(())
    }

    async fn delete(&self, url: &str) -> Result<()> {
        let resp = self
            .client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        Ok(())
    }

    async fn put_bytes<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<T> {
        let resp = self
            .client
            .put(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", content_type)
            .body(bytes)
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

    pub async fn get_messages(&self, chat_id: &str) -> Result<(Vec<Message>, Option<String>)> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages?\
             $top=50&$orderby=createdDateTime%20desc",
            chat_id
        );
        let resp: PagedResponse<Message> = self.get(&url).await?;
        let mut messages = resp.value;
        messages.reverse(); // Show oldest first
        Ok((messages, resp.next_link))
    }

    /// Fetch only new/changed messages since the last delta token.
    /// On first call pass None for delta_link to get initial state + token.
    pub async fn get_messages_delta(
        &self,
        chat_id: &str,
        delta_link: Option<&str>,
    ) -> Result<(Vec<Message>, Option<String>)> {
        let url = match delta_link {
            Some(link) => link.to_string(),
            None => format!(
                "https://graph.microsoft.com/v1.0/me/chats/{}/messages/delta",
                chat_id
            ),
        };
        // Follow all pages to collect complete delta
        let mut all_messages = Vec::new();
        let mut current_url = url;
        loop {
            let resp: DeltaResponse<Message> = self.get(&current_url).await?;
            all_messages.extend(resp.value);
            if let Some(next) = resp.next_link {
                current_url = next;
            } else {
                return Ok((all_messages, resp.delta_link));
            }
        }
    }

    pub async fn get_messages_page(&self, next_link: &str) -> Result<(Vec<Message>, Option<String>)> {
        let resp: PagedResponse<Message> = self.get(next_link).await?;
        let mut messages = resp.value;
        messages.reverse();
        Ok((messages, resp.next_link))
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

    pub async fn send_reply(
        &self,
        chat_id: &str,
        reply_to_id: &str,
        content: &str,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages",
            chat_id
        );
        let body = serde_json::json!({
            "body": {
                "content": content,
                "contentType": "text"
            },
            "replyToId": reply_to_id
        });
        self.post_json(&url, &body).await
    }

    pub async fn update_message(
        &self,
        chat_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages/{}",
            chat_id, message_id
        );
        let body = serde_json::json!({
            "body": {
                "content": content,
                "contentType": "text"
            }
        });
        self.patch_json(&url, &body).await
    }

    pub async fn soft_delete_message(&self, chat_id: &str, message_id: &str) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages/{}/softDelete",
            chat_id, message_id
        );
        self.post_no_response(&url).await
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
        let escaped = query.replace('\'', "''");
        let url = format!(
            "https://graph.microsoft.com/v1.0/users?\
             $filter=startswith(displayName,'{}') or startswith(mail,'{}') or startswith(userPrincipalName,'{}')&\
             $top=8&$select=id,displayName,mail,userPrincipalName",
            escaped, escaped, escaped
        );
        let resp: GraphResponse<User> = self.get(&url).await?;
        Ok(resp.value)
    }

    // ---- Chat management ----

    pub async fn get_chat_members(&self, chat_id: &str) -> Result<Vec<ChatMember>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/members",
            chat_id
        );
        let resp: GraphResponse<ChatMember> = self.get(&url).await?;
        Ok(resp.value)
    }

    pub async fn rename_chat(&self, chat_id: &str, topic: &str) -> Result<()> {
        let url = format!("https://graph.microsoft.com/v1.0/chats/{}", chat_id);
        let body = serde_json::json!({ "topic": topic });
        self.patch_no_content(&url, &body).await
    }

    pub async fn add_chat_member(
        &self,
        chat_id: &str,
        user_id: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/chats/{}/members",
            chat_id
        );
        let body = serde_json::json!({
            "@odata.type": "#microsoft.graph.aadUserConversationMember",
            "roles": ["owner"],
            "user@odata.bind": format!("https://graph.microsoft.com/v1.0/users('{}')", user_id)
        });
        self.post_no_content(&url, &body).await
    }

    pub async fn remove_chat_member(
        &self,
        chat_id: &str,
        membership_id: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/chats/{}/members/{}",
            chat_id, membership_id
        );
        self.delete(&url).await
    }

    // ---- Reactions ----

    pub async fn set_reaction(
        &self,
        chat_id: &str,
        message_id: &str,
        reaction_type: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/chats/{}/messages/{}/setReaction",
            chat_id, message_id
        );
        let body = serde_json::json!({ "reactionType": reaction_type });
        self.post_no_content(&url, &body).await
    }

    pub async fn set_channel_reaction(
        &self,
        team_id: &str,
        channel_id: &str,
        message_id: &str,
        reaction_type: &str,
    ) -> Result<()> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages/{}/setReaction",
            team_id, channel_id, message_id
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
            "https://graph.microsoft.com/v1.0/chats/{}/messages/{}/unsetReaction",
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

    pub async fn get_channel_members(
        &self,
        team_id: &str,
        channel_id: &str,
    ) -> Result<Vec<ChannelMember>> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/members",
            team_id, channel_id
        );
        let resp: GraphResponse<ChannelMember> = self.get(&url).await?;
        Ok(resp.value)
    }

    pub async fn get_channel_messages(
        &self,
        team_id: &str,
        channel_id: &str,
    ) -> Result<(Vec<Message>, Option<String>)> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages?$top=50",
            team_id, channel_id
        );
        let resp: PagedResponse<Message> = self.get(&url).await?;
        let mut messages = resp.value;
        messages.reverse();
        Ok((messages, resp.next_link))
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

    // ---- File Upload ----

    /// Upload a file to OneDrive (Microsoft Teams Chat Files folder) and return the DriveItem
    pub async fn upload_file(&self, filename: &str, bytes: Vec<u8>) -> Result<DriveItem> {
        let encoded_name = filename.replace('\'', "''");
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/drive/root:/Microsoft Teams Chat Files/{}:/content",
            encoded_name
        );
        self.put_bytes(&url, bytes, "application/octet-stream").await
    }

    /// Send a chat message with a file attachment reference
    pub async fn send_message_with_attachment(
        &self,
        chat_id: &str,
        filename: &str,
        drive_item: &DriveItem,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/chats/{}/messages",
            chat_id
        );
        let raw_tag = drive_item.e_tag.as_deref().unwrap_or(&drive_item.id);
        let attachment_id = raw_tag.trim_matches('"');
        let body = serde_json::json!({
            "body": {
                "contentType": "html",
                "content": format!("<attachment id=\"{}\"></attachment>", attachment_id)
            },
            "attachments": [{
                "id": attachment_id,
                "contentType": "reference",
                "contentUrl": drive_item.web_url,
                "name": filename
            }]
        });
        self.post_json(&url, &body).await
    }

    /// Send a channel message with a file attachment reference
    pub async fn send_channel_message_with_attachment(
        &self,
        team_id: &str,
        channel_id: &str,
        filename: &str,
        drive_item: &DriveItem,
    ) -> Result<Message> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/teams/{}/channels/{}/messages",
            team_id, channel_id
        );
        let raw_tag = drive_item.e_tag.as_deref().unwrap_or(&drive_item.id);
        let attachment_id = raw_tag.trim_matches('"');
        let body = serde_json::json!({
            "body": {
                "contentType": "html",
                "content": format!("<attachment id=\"{}\"></attachment>", attachment_id)
            },
            "attachments": [{
                "id": attachment_id,
                "contentType": "reference",
                "contentUrl": drive_item.web_url,
                "name": filename
            }]
        });
        self.post_json(&url, &body).await
    }

    // ---- Search ----

    pub async fn search_messages(&self, query: &str) -> Result<Vec<SearchHit>> {
        let body = serde_json::json!({
            "requests": [{
                "entityTypes": ["chatMessage"],
                "query": { "queryString": query },
                "from": 0,
                "size": 25
            }]
        });
        let resp: SearchResponse = self
            .post_json("https://graph.microsoft.com/v1.0/search/query", &body)
            .await?;
        let hits = resp
            .value
            .into_iter()
            .flat_map(|rs| rs.hits_containers)
            .flat_map(|hc| hc.hits)
            .collect();
        Ok(hits)
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
