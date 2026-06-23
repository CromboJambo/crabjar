//! Types for Graph API responses and requests.
use serde::{Deserialize, Serialize};

/// Calendar event from Microsoft Graph API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub subject: Option<String>,
    pub body_preview: Option<String>,
    pub start: EventDateTime,
    pub end: EventDateTime,
    pub organizer: Option<EventOrganizer>,
    pub attendees: Option<Vec<EventAttendee>>,
    pub location: Option<EventLocation>,
    pub is_online_meeting: Option<bool>,
    pub online_meeting_url: Option<String>,
    pub created_on: Option<String>,
}

/// Event date/time from Graph API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDateTime {
    pub date_time: String,
    pub timezone: String,
}

/// Event organizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOrganizer {
    pub user: Option<EmailAddress>,
}

/// Event attendee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAttendee {
    pub type_: String,
    pub status: Option<EventStatus>,
    pub email_address: Option<EmailAddress>,
}

/// Event status (acceptance response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStatus {
    pub response: String,
    pub time: Option<String>,
}

/// Email address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub address: String,
    pub name: Option<String>,
}

/// Event location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLocation {
    pub display_name: Option<String>,
    pub location_type: Option<String>,
    pub unique_id: Option<String>,
}

/// Mail message from Graph API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub id: String,
    pub subject: Option<String>,
    pub body_preview: Option<String>,
    pub from_: Option<MailSender>,
    pub sent_datetime: Option<String>,
    pub has_attachments: Option<bool>,
    pub is_read: Option<bool>,
}

/// Mail sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailSender {
    pub email_address: Option<EmailAddress>,
}

/// People search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonResult {
    pub person_id: Option<String>,
    pub display_name: Option<String>,
    pub email_address: Option<String>,
    pub given_name: Option<String>,
    pub surname: Option<String>,
    pub company_name: Option<String>,
    pub department: Option<String>,
    #[serde(rename = "scoredEmailAddresses")]
    pub scored_email_addresses: Option<Vec<ScoredEmailAddress>>,
}

/// Scored email address (People API relevance ranking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEmailAddress {
    pub address: String,
    pub relevance_score: Option<f64>,
}

/// User profile from `/me`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub user_principal_name: Option<String>,
    pub mail: Option<String>,
    pub given_name: Option<String>,
    pub surname: Option<String>,
}

/// Chat message send request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRequest {
    pub body: MessageBody,
}

/// Message body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBody {
    pub content: String,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

impl ChatMessageRequest {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            body: MessageBody {
                content: content.into(),
                content_type: Some("text".to_string()),
            },
        }
    }
}

/// Generic paginated response wrapper (Graph API `value` array).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    pub next_link: Option<String>,
}

/// Calendar events response.
pub type CalendarEventsResponse = PaginatedResponse<CalendarEvent>;

/// Mail messages response.
pub type MailMessagesResponse = PaginatedResponse<MailMessage>;

/// People search response.
pub type PeopleResponse = PaginatedResponse<PersonResult>;

/// Unified API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphApiResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub status: Option<u16>,
}

impl<T> GraphApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            status: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            status: None,
        }
    }
}
