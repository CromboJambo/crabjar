/// Microsoft Graph API client.
///
/// Mirrors the Teams-for-Linux `GraphApiClient` implementation:
/// - Token acquisition via a `TokenProvider` trait
/// - Token caching with 5-minute expiry buffer
/// - Calendar, mail, people, and chat endpoints
/// - `OData` query parameter building
use std::time::{Duration, Instant};

use anyhow::anyhow;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use tracing::{debug, warn};

use crate::config::GraphApiConfig;
use crate::types::{
    CalendarEvent, CalendarEventsResponse, ChatMessageRequest, GraphApiResponse,
    MailMessagesResponse, PeopleResponse, UserProfile,
};

/// Trait for acquiring Microsoft Graph API tokens.
///
/// Implementations typically wrap the `WebView` session manager's
/// token acquisition (which communicates with the Electron Chromium session).
#[async_trait]
pub trait TokenProvider: Send + Sync {
    /// Acquire a token for the given resource/scopes.
    ///
    /// Returns `Some(token)` if successful, `None` if acquisition failed.
    /// `force_refresh` bypasses the provider's own cache.
    async fn acquire_token(&self, resource: &str, force_refresh: bool) -> Option<String>;
}

/// Token cache entry.
#[derive(Debug, Clone)]
struct TokenEntry {
    token: String,
    acquired_at: Instant,
    expires_at: Option<Instant>,
}

impl TokenEntry {
    /// Check if the token is still valid (with 5-minute buffer).
    #[allow(clippy::duration_suboptimal_units)]
    fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(expiry) => {
                let buffer = Duration::from_secs(300);
                let elapsed = Instant::now().duration_since(self.acquired_at);
                let token_duration = expiry - self.acquired_at;
                elapsed < token_duration.checked_sub(buffer).unwrap_or(Duration::ZERO)
            }
            None => true,
        }
    }
}

/// Microsoft Graph API client.
///
/// Provides typed access to Microsoft Graph endpoints for calendar, mail,
/// people search, and chat. Token acquisition is delegated to a
/// `TokenProvider` implementation.
pub struct GraphApiClient {
    config: GraphApiConfig,
    http_client: HttpClient,
    token_cache: Option<TokenEntry>,
}

impl GraphApiClient {
    /// Create a new Graph API client.
    #[must_use]
    pub fn new(config: GraphApiConfig) -> Self {
        Self {
            config,
            http_client: HttpClient::new(),
            token_cache: None,
        }
    }

    /// Check if the client is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Acquire a token from the provider, with caching.
    async fn acquire_token<P: TokenProvider>(
        &mut self,
        provider: &P,
        force_refresh: bool,
    ) -> Option<String> {
        if !force_refresh
            && let Some(ref entry) = self.token_cache
            && entry.is_valid()
        {
            debug!(
                time_until_expiry = ?entry.expires_at.map(|e| e.saturating_duration_since(Instant::now())),
                "Using cached Graph API token"
            );
            return Some(entry.token.clone());
        }

        let token = provider
            .acquire_token("https://graph.microsoft.com", force_refresh)
            .await?;

        self.token_cache = Some(TokenEntry {
            token: token.clone(),
            acquired_at: Instant::now(),
            expires_at: Some(Instant::now() + Duration::from_hours(1)),
        });

        debug!("Graph API token acquired and cached");
        Some(token)
    }

    /// Build `OData` query string from options.
    fn build_odata_query(params: &std::collections::HashMap<String, String>) -> String {
        let supported = [
            "startDateTime",
            "endDateTime",
            "$top",
            "$select",
            "$filter",
            "$orderby",
            "$skip",
            "$count",
            "$search",
            "$expand",
        ];

        let query: Vec<_> = params
            .iter()
            .filter(|(k, v)| supported.contains(&k.as_str()) && !v.is_empty())
            .map(|(k, v)| format!("{k}={v}"))
            .collect();

        query.join("&")
    }

    /// Make an authenticated request to the Graph API.
    async fn make_request<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        endpoint: &str,
        method: &reqwest::Method,
        query_params: &std::collections::HashMap<String, String>,
        headers: &std::collections::HashMap<String, String>,
        body: Option<serde_json::Value>,
    ) -> anyhow::Result<reqwest::Response> {
        if !self.config.enabled {
            warn!("Graph API is disabled");
            return Err(anyhow!("Graph API is disabled"));
        }

        let Some(token) = self.acquire_token(provider, false).await else {
            return Err(anyhow!("Failed to acquire token"));
        };

        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}{}", self.config.base_url, endpoint)
        };

        let query = Self::build_odata_query(query_params);
        let url = if query.is_empty() {
            url
        } else {
            format!("{url}?{query}")
        };

        debug!(method = ?method, endpoint = %url, "Making Graph API request");

        let mut request = self
            .http_client
            .request(method.clone(), &url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json");

        for (key, value) in headers {
            request = request.header(key, value);
        }

        if let Some(ref body) = body {
            request = request.json(body);
        }

        request.send().await.map_err(|e| anyhow!(e))
    }

    /// Get current user profile from Graph API (`/me`).
    pub async fn get_user_profile<P: TokenProvider>(
        &mut self,
        provider: &mut P,
    ) -> GraphApiResponse<UserProfile> {
        match self
            .make_request(
                provider,
                "/me",
                &reqwest::Method::GET,
                &Default::default(),
                &Default::default(),
                None,
            )
            .await
        {
            Ok(resp) => {
                let data: UserProfile = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse user profile: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(
                    display_name = %data.display_name.as_deref().unwrap_or("?"),
                    "User profile retrieved"
                );
                GraphApiResponse::ok(data)
            }
            Err(e) => {
                warn!("Graph API request failed: {}", e);
                GraphApiResponse::err(format!("Request failed: {e}"))
            }
        }
    }

    /// Get calendar events with optional `OData` query options.
    pub async fn get_calendar_events<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        params: std::collections::HashMap<String, String>,
    ) -> GraphApiResponse<CalendarEventsResponse> {
        let query = Self::build_odata_query(&params);
        let endpoint = if query.is_empty() {
            "/me/calendar/events".to_string()
        } else {
            format!("/me/calendar/events?{query}")
        };

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::GET,
                &params,
                &Default::default(),
                None,
            )
            .await
        {
            Ok(resp) => {
                let data: CalendarEventsResponse = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse calendar events: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(event_count = data.value.len(), "Calendar events retrieved");
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Get calendar view for a time range (ISO 8601 date strings).
    pub async fn get_calendar_view<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        start_datetime: impl Into<String>,
        end_datetime: impl Into<String>,
        extra_params: std::collections::HashMap<String, String>,
    ) -> GraphApiResponse<CalendarEventsResponse> {
        let mut params = extra_params;
        params.insert("startDateTime".to_string(), start_datetime.into());
        params.insert("endDateTime".to_string(), end_datetime.into());

        let query = Self::build_odata_query(&params);
        let endpoint = if query.is_empty() {
            "/me/calendar/calendarView".to_string()
        } else {
            format!("/me/calendar/calendarView?{query}")
        };

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::GET,
                &params,
                &Default::default(),
                None,
            )
            .await
        {
            Ok(resp) => {
                let data: CalendarEventsResponse = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse calendar view: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(event_count = data.value.len(), "Calendar view retrieved");
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Create a calendar event.
    pub async fn create_calendar_event<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        event: serde_json::Value,
    ) -> GraphApiResponse<CalendarEvent> {
        match self
            .make_request(
                provider,
                "/me/calendar/events",
                &reqwest::Method::POST,
                &Default::default(),
                &Default::default(),
                Some(event),
            )
            .await
        {
            Ok(resp) => {
                let data: CalendarEvent = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse created event: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(
                    event_id = %data.id,
                    subject = %data.subject.as_deref().unwrap_or("?"),
                    "Calendar event created"
                );
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Update a calendar event by ID.
    pub async fn update_calendar_event<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        event_id: impl Into<String>,
        updates: serde_json::Value,
    ) -> GraphApiResponse<CalendarEvent> {
        let event_id_str = event_id.into();
        let endpoint = format!("/me/calendar/events/{event_id_str}");

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::PATCH,
                &Default::default(),
                &Default::default(),
                Some(updates),
            )
            .await
        {
            Ok(resp) => {
                let data: CalendarEvent = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse updated event: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(event_id = %event_id_str, "Calendar event updated");
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Delete a calendar event by ID.
    pub async fn delete_calendar_event<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        event_id: impl Into<String>,
    ) -> GraphApiResponse<serde_json::Value> {
        let event_id_str = event_id.into();
        let endpoint = format!("/me/calendar/events/{event_id_str}");

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::DELETE,
                &Default::default(),
                &Default::default(),
                None,
            )
            .await
        {
            Ok(_resp) => {
                debug!(event_id = %event_id_str, "Calendar event deleted");
                GraphApiResponse::ok(serde_json::Value::Null)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Get mail messages with optional `OData` query options.
    pub async fn get_mail_messages<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        params: std::collections::HashMap<String, String>,
    ) -> GraphApiResponse<MailMessagesResponse> {
        let query = Self::build_odata_query(&params);
        let endpoint = if query.is_empty() {
            "/me/messages".to_string()
        } else {
            format!("/me/messages?{query}")
        };

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::GET,
                &params,
                &Default::default(),
                None,
            )
            .await
        {
            Ok(resp) => {
                let data: MailMessagesResponse = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse mail messages: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(message_count = data.value.len(), "Mail messages retrieved");
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Search people using the People API.
    pub async fn search_people<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        query: impl Into<String>,
        extra_params: std::collections::HashMap<String, String>,
    ) -> GraphApiResponse<PeopleResponse> {
        let q = query.into();
        let mut params = extra_params;
        if !q.is_empty() {
            // Escape quotes and backslashes (mirrors Electron app)
            let escaped = q.replace('\\', "\\\\").replace('"', "\\\"");
            params.insert("$search".to_string(), format!("\"{escaped}\""));
        }

        let query_str = Self::build_odata_query(&params);
        let endpoint = if query_str.is_empty() {
            "/me/people".to_string()
        } else {
            format!("/me/people?{query_str}")
        };

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::GET,
                &params,
                &Default::default(),
                None,
            )
            .await
        {
            Ok(resp) => {
                let data: PeopleResponse = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to parse people results: {}", e);
                        return GraphApiResponse::err(format!("Failed to parse response: {e}"));
                    }
                };
                debug!(result_count = data.value.len(), "People search results");
                GraphApiResponse::ok(data)
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }

    /// Send a chat message to a chat thread.
    pub async fn send_chat_message<P: TokenProvider>(
        &mut self,
        provider: &mut P,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> GraphApiResponse<()> {
        let chat_id_str = chat_id.into();
        let message = ChatMessageRequest::new(content);
        let body = serde_json::to_value(message).unwrap_or_default();

        let endpoint = format!("/chats/{chat_id_str}/messages");

        match self
            .make_request(
                provider,
                &endpoint,
                &reqwest::Method::POST,
                &Default::default(),
                &Default::default(),
                Some(body),
            )
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    debug!(chat_id = %chat_id_str, "Chat message sent");
                    GraphApiResponse::ok(())
                } else {
                    let status = resp.status().as_u16();
                    let error_text = resp.text().await.unwrap_or_default();
                    warn!(
                        chat_id = %chat_id_str,
                        status,
                        error = %error_text.chars().take(100).collect::<String>(),
                        "Send message failed"
                    );
                    GraphApiResponse::err(format!(
                        "API returned status {}: {}",
                        status,
                        error_text.chars().take(200).collect::<String>()
                    ))
                }
            }
            Err(e) => GraphApiResponse::err(format!("Request failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_odata_query_empty() {
        let params = std::collections::HashMap::new();
        let query = GraphApiClient::build_odata_query(&params);
        assert!(query.is_empty());
    }

    #[test]
    fn test_build_odata_query_with_params() {
        let mut params = std::collections::HashMap::new();
        params.insert("$top".to_string(), "10".to_string());
        params.insert("$select".to_string(), "subject,start,end".to_string());
        let query = GraphApiClient::build_odata_query(&params);
        assert!(query.contains("$top=10"));
        assert!(query.contains("$select=subject,start,end"));
    }

    #[test]
    fn test_build_odata_query_ignores_unsupported() {
        let mut params = std::collections::HashMap::new();
        params.insert("customParam".to_string(), "value".to_string());
        let query = GraphApiClient::build_odata_query(&params);
        assert!(query.is_empty());
    }

    #[test]
    fn test_chat_message_request() {
        let req = ChatMessageRequest::new("Hello world");
        assert_eq!(req.body.content, "Hello world");
        assert_eq!(req.body.content_type, Some("text".to_string()));
    }

    #[test]
    fn test_graph_api_response_ok() {
        let resp: GraphApiResponse<String> = GraphApiResponse::ok("data".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("data".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_graph_api_response_err() {
        let resp: GraphApiResponse<String> = GraphApiResponse::err("error msg");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("error msg".to_string()));
    }
}
