use anyhow::{Context, Result};
use zeroize::Zeroize;

use crate::logging;
use crate::models::*;

fn looks_like_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) // PNG
        || bytes.starts_with(&[0xFF, 0xD8, 0xFF]) // JPEG
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(b"BM") // BMP
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
}

fn append_query_hint(url: &str, key: &str, value: &str) -> String {
    if url.contains('?') {
        format!("{url}&{key}={value}")
    } else {
        format!("{url}?{key}={value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryDownloadFailure {
    Transport,
    Http401,
    Http403,
    Http404,
    Http4xx,
    Http5xx,
    HttpOther,
    ReadBody,
    NonImageBody,
}

impl BinaryDownloadFailure {
    fn from_http_status(code: u16) -> Self {
        match code {
            401 => BinaryDownloadFailure::Http401,
            403 => BinaryDownloadFailure::Http403,
            404 => BinaryDownloadFailure::Http404,
            400..=499 => BinaryDownloadFailure::Http4xx,
            500..=599 => BinaryDownloadFailure::Http5xx,
            _ => BinaryDownloadFailure::HttpOther,
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            BinaryDownloadFailure::Transport => "image_preview.download.transport",
            BinaryDownloadFailure::Http401 => "image_preview.download.http_401",
            BinaryDownloadFailure::Http403 => "image_preview.download.http_403",
            BinaryDownloadFailure::Http404 => "image_preview.download.http_404",
            BinaryDownloadFailure::Http4xx => "image_preview.download.http_4xx",
            BinaryDownloadFailure::Http5xx => "image_preview.download.http_5xx",
            BinaryDownloadFailure::HttpOther => "image_preview.download.http_other",
            BinaryDownloadFailure::ReadBody => "image_preview.download.read_body",
            BinaryDownloadFailure::NonImageBody => "image_preview.download.non_image",
        }
    }
}

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
        let resp = match self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.get.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.get.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        match resp
            .json::<T>()
            .await
            .context("Failed to parse Graph API response")
        {
            Ok(value) => {
                logging::try_log_event("graph.get.success");
                Ok(value)
            }
            Err(e) => {
                logging::try_log_failure("graph.get.parse");
                Err(e)
            }
        }
    }

    pub async fn download_binary_with_reason(
        &self,
        url: &str,
    ) -> std::result::Result<Vec<u8>, BinaryDownloadFailure> {
        let mut candidate_urls = vec![url.to_string()];
        for hinted in [
            append_query_hint(url, "download", "1"),
            append_query_hint(url, "raw", "1"),
        ] {
            if !candidate_urls.contains(&hinted) {
                candidate_urls.push(hinted);
            }
        }

        let mut last_failure = BinaryDownloadFailure::Transport;
        for candidate in candidate_urls {
            let resp = self
                .client
                .get(&candidate)
                .header("Authorization", format!("Bearer {}", self.access_token))
                .header("Accept", "image/*,*/*;q=0.8")
                .send()
                .await;
            let resp = match resp {
                Ok(r) => r,
                Err(_) => {
                    last_failure = BinaryDownloadFailure::Transport;
                    continue;
                }
            };
            let status = resp.status();
            if !status.is_success() {
                let _ = resp.text().await;
                last_failure = BinaryDownloadFailure::from_http_status(status.as_u16());
                continue;
            }
            let bytes = match resp.bytes().await {
                Ok(b) => b.to_vec(),
                Err(_) => {
                    last_failure = BinaryDownloadFailure::ReadBody;
                    continue;
                }
            };
            if looks_like_image_bytes(&bytes) {
                return Ok(bytes);
            }
            last_failure = BinaryDownloadFailure::NonImageBody;
        }

        Err(last_failure)
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let resp = match self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.post_json.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.post_json.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        match resp
            .json::<T>()
            .await
            .context("Failed to parse Graph API response")
        {
            Ok(value) => {
                logging::try_log_event("graph.post_json.success");
                Ok(value)
            }
            Err(e) => {
                logging::try_log_failure("graph.post_json.parse");
                Err(e)
            }
        }
    }

    async fn post_no_content(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let resp = match self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.post_no_content.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.post_no_content.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        logging::try_log_event("graph.post_no_content.success");
        Ok(())
    }

    async fn patch_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let resp = match self
            .client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.patch_json.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.patch_json.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        match resp
            .json::<T>()
            .await
            .context("Failed to parse Graph API response")
        {
            Ok(value) => {
                logging::try_log_event("graph.patch_json.success");
                Ok(value)
            }
            Err(e) => {
                logging::try_log_failure("graph.patch_json.parse");
                Err(e)
            }
        }
    }

    async fn post_no_response(&self, url: &str) -> Result<()> {
        let resp = match self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Length", "0")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.post_no_response.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.post_no_response.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        logging::try_log_event("graph.post_no_response.success");
        Ok(())
    }

    async fn patch_no_content(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let resp = match self
            .client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.patch_no_content.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.patch_no_content.http");
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, text);
        }
        logging::try_log_event("graph.patch_no_content.success");
        Ok(())
    }

    async fn delete(&self, url: &str) -> Result<()> {
        let resp = match self
            .client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.delete.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.delete.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        logging::try_log_event("graph.delete.success");
        Ok(())
    }

    async fn put_bytes<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<T> {
        let resp = match self
            .client
            .put(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", content_type)
            .body(bytes)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                logging::try_log_failure("graph.put_bytes.transport");
                return Err(e.into());
            }
        };
        let status = resp.status();
        if !status.is_success() {
            logging::try_log_failure("graph.put_bytes.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Graph API error ({}): {}", status, body);
        }
        match resp
            .json::<T>()
            .await
            .context("Failed to parse Graph API response")
        {
            Ok(value) => {
                logging::try_log_event("graph.put_bytes.success");
                Ok(value)
            }
            Err(e) => {
                logging::try_log_failure("graph.put_bytes.parse");
                Err(e)
            }
        }
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

    pub async fn get_messages_page(
        &self,
        next_link: &str,
    ) -> Result<(Vec<Message>, Option<String>)> {
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

    pub async fn add_chat_member(&self, chat_id: &str, user_id: &str) -> Result<()> {
        let url = format!("https://graph.microsoft.com/v1.0/chats/{}/members", chat_id);
        let body = serde_json::json!({
            "@odata.type": "#microsoft.graph.aadUserConversationMember",
            "roles": ["owner"],
            "user@odata.bind": format!("https://graph.microsoft.com/v1.0/users('{}')", user_id)
        });
        self.post_no_content(&url, &body).await
    }

    pub async fn remove_chat_member(&self, chat_id: &str, membership_id: &str) -> Result<()> {
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

    pub async fn set_my_presence(&self, availability: &str, activity: &str) -> Result<()> {
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
        self.put_bytes(&url, bytes, "application/octet-stream")
            .await
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
        let attachment_id = uuid::Uuid::new_v4().to_string();
        let body = serde_json::json!({
            "body": {
                "contentType": "html",
                "content": format!("{} <attachment id=\"{}\"></attachment>", filename, attachment_id)
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
        let attachment_id = uuid::Uuid::new_v4().to_string();
        let body = serde_json::json!({
            "body": {
                "contentType": "html",
                "content": format!("{} <attachment id=\"{}\"></attachment>", filename, attachment_id)
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
            .await
            .map_err(|e| {
                logging::try_log_failure("graph.mark_chat_read.transport");
                e
            })?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 204 {
            logging::try_log_failure("graph.mark_chat_read.http");
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to mark chat read ({}): {}", status, body);
        }
        logging::try_log_event("graph.mark_chat_read.success");
        Ok(())
    }
}

impl Drop for GraphClient {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::{append_query_hint, looks_like_image_bytes, BinaryDownloadFailure};
    use crate::logging::is_safe_event_label;

    #[test]
    fn detects_png_signature() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 1];
        assert!(looks_like_image_bytes(&png));
    }

    #[test]
    fn rejects_html_signature() {
        let html = b"<html><body>not image</body></html>";
        assert!(!looks_like_image_bytes(html));
    }

    #[test]
    fn appends_query_hint_correctly() {
        assert_eq!(
            append_query_hint("https://example.com/file.png", "download", "1"),
            "https://example.com/file.png?download=1"
        );
        assert_eq!(
            append_query_hint("https://example.com/file.png?x=1", "download", "1"),
            "https://example.com/file.png?x=1&download=1"
        );
    }

    #[test]
    fn download_failure_labels_are_safe() {
        let labels = [
            BinaryDownloadFailure::Transport.as_label(),
            BinaryDownloadFailure::Http401.as_label(),
            BinaryDownloadFailure::Http403.as_label(),
            BinaryDownloadFailure::Http404.as_label(),
            BinaryDownloadFailure::Http4xx.as_label(),
            BinaryDownloadFailure::Http5xx.as_label(),
            BinaryDownloadFailure::HttpOther.as_label(),
            BinaryDownloadFailure::ReadBody.as_label(),
            BinaryDownloadFailure::NonImageBody.as_label(),
        ];
        assert!(labels.into_iter().all(is_safe_event_label));
    }
}
