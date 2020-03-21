use crate::models::Event as GenericEvent;
use crate::models::Post as GenericPost;
use crate::models::Status as GenericStatus;
use crate::models::StatusCode;
use crate::models::StatusError as GenericStatusError;
use serde::{Deserialize, Serialize};
use std::convert::Into;

#[derive(Deserialize, Serialize)]
pub struct Posted {
    channel_display_name: String,
    channel_name: String,
    channel_type: String,
    post: String,
    sender_name: String,
    team_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Status {
    pub status: String,
    pub error: Option<StatusDetails>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StatusDetails {
    id: String,
    message: String,
    detailed_error: String,
    request_id: Option<String>,
    status_code: f64,
    is_oauth: Option<bool>,
}

impl Into<GenericPost> for Posted {
    fn into(self) -> GenericPost {
        // FIXME: must still decode self.post
        GenericPost {
            user_id: self.sender_name,
            root_id: self.post.clone(),
            parent_id: "".to_string(),
            message: self.post.clone(),
            channel_id: self.channel_name,
        }
    }
}

impl Into<GenericStatusError> for StatusDetails {
    fn into(self) -> GenericStatusError {
        GenericStatusError {
            message: self.message,
            detailed_error: self.detailed_error,
            request_id: self.request_id,
            status_code: self.status_code as i32,
        }
    }
}

impl Into<GenericStatus> for Status {
    fn into(self) -> GenericStatus {
        if self.status.contains("OK") {
            return GenericStatus {
                code: StatusCode::OK,
                error: None,
            };
        }

        if self.status.contains("FAIL") {
            return GenericStatus {
                code: StatusCode::Error,
                error: Some(self.error.unwrap().into()),
            };
        }

        GenericStatus {
            code: StatusCode::Unsupported,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventData {
    Posted(Posted),
}

#[derive(Serialize, Deserialize)]
pub struct Event {
    #[serde(rename(serialize = "event", deserialize = "event"))]
    type_: String,
    data: EventData,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetaEvent {
    Status(Status),
    Event(Event),
    Unsupported(String),
}

impl Into<GenericEvent> for Event {
    fn into(self) -> GenericEvent {
        match self.data {
            EventData::Posted(posted) => GenericEvent::Post(posted.into()),
        }
    }
}

impl Into<GenericEvent> for Status {
    fn into(self) -> GenericEvent {
        GenericEvent::Status(self.into())
    }
}

impl Into<GenericEvent> for MetaEvent {
    fn into(self) -> GenericEvent {
        match self {
            MetaEvent::Event(event) => event.into(),
            MetaEvent::Status(status) => status.into(),
            MetaEvent::Unsupported(unsupported) => GenericEvent::Unsupported(unsupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_valid() {
        let data = r#"{"event": "posted", "data": {"channel_display_name":"Town Square","channel_name":"town-square","channel_type":"O","post":"{\"id\":\"ghkm74cqzbnjxr5dx638k73xqa\",\"create_at\":1576937676623,\"update_at\":1576937676623,\"edit_at\":0,\"delete_at\":0,\"is_pinned\":false,\"user_id\":\"kh9859j8kir15dmxonsm8sxq1w\",\"channel_id\":\"amtak96j3br5iyokgunmf188jc\",\"root_id\":\"\",\"parent_id\":\"\",\"original_id\":\"\",\"message\":\"test\",\"type\":\"\",\"props\":{},\"hashtags\":\"\",\"pending_post_id\":\"kh9859j8kir15dmxonsm8sxq1w:1576937676569\",\"metadata\":{}}","sender_name":"@admin","team_id":"49ck75z1figmpjy6eknrohsjnw"}, "broadcast": {"omit_users":null,"user_id":"","channel_id":"amtak96j3br5iyokgunmf188jc","team_id":""}, "seq": 7}"#;
        let valid: MetaEvent = serde_json::from_str(data).unwrap();
        let event = match valid {
            MetaEvent::Event(event) => event,
            _ => panic!("wrong type"),
        };

        assert_eq!(event.type_, "posted");

        match event.data {
            EventData::Posted(event) => {
                assert_eq!(event.channel_display_name, "Town Square");
                assert_eq!(event.channel_name, "town-square");
                assert_eq!(event.channel_type, "O");
                assert_ne!(event.post, "");
            }
        }
    }

    #[test]
    #[should_panic]
    fn post_invalid() {
        let data = r#"{"event": "posted", "data": {"invalid":"invalid"}}"#;
        let _invalid: MetaEvent = serde_json::from_str(data).unwrap();
    }

    #[test]
    fn app_error() {
        let data = r#"{"status": "FAIL", "error": {"id": "api.web_socket_router.bad_seq.app_error", "message": "Invalid sequence for WebSocket message.", "detailed_error": "", "status_code": 400}}"#;
        let valid: MetaEvent = serde_json::from_str(data).unwrap();
        let status = match valid {
            MetaEvent::Status(status) => status,
            _ => panic!("wrong type"),
        };

        assert_eq!("FAIL", status.status);
    }
}