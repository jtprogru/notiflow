//! Building the JSON body for `sendMessage` and `editMessageText`.

use serde_json::{Map, Value, json};

use crate::model::{ChatId, MessageId, ParseMode, ThreadId};

/// Everything needed to produce one Bot API call.
#[derive(Debug, Clone)]
pub struct Request {
    pub chat_id: ChatId,
    pub text: String,
    pub parse_mode: ParseMode,
    pub disable_web_page_preview: bool,
    pub disable_notification: bool,
    pub message_thread_id: Option<ThreadId>,
    /// When set, the call becomes an edit of that message instead of a fresh send.
    pub edit_message_id: Option<MessageId>,
}

impl Request {
    /// `sendMessage` or `editMessageText`, depending on whether an id is being edited.
    pub fn endpoint(&self) -> &'static str {
        if self.edit_message_id.is_some() { "editMessageText" } else { "sendMessage" }
    }

    /// The JSON body.
    ///
    /// Link previews are expressed as `link_preview_options.is_disabled`. Telegram
    /// deprecated the flat `disable_web_page_preview` field; the notiflow input keeps its
    /// old name, only the wire representation moved.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("chat_id".into(), self.chat_id.to_json());
        map.insert("text".into(), Value::from(self.text.clone()));
        map.insert(
            "link_preview_options".into(),
            json!({ "is_disabled": self.disable_web_page_preview }),
        );
        if let Some(pm) = self.parse_mode.wire_value() {
            map.insert("parse_mode".into(), Value::from(pm));
        }

        match self.edit_message_id {
            Some(id) => {
                // Telegram rejects disable_notification and message_thread_id on edits, so
                // they are dropped rather than passed through and 400'd.
                map.insert("message_id".into(), Value::from(id.get()));
            }
            None => {
                map.insert("disable_notification".into(), Value::from(self.disable_notification));
                if let Some(thread) = self.message_thread_id {
                    map.insert("message_thread_id".into(), Value::from(thread.get()));
                }
            }
        }

        Value::Object(map)
    }

    /// Compact single-line JSON, the form actually posted.
    pub fn to_body(&self) -> String {
        self.to_json().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Request {
        Request {
            chat_id: "-1001234567890".parse().unwrap(),
            text: "hello".into(),
            parse_mode: ParseMode::MarkdownV2,
            disable_web_page_preview: true,
            disable_notification: false,
            message_thread_id: None,
            edit_message_id: None,
        }
    }

    #[test]
    fn send_shape() {
        let v = base().to_json();
        assert_eq!(v["chat_id"], json!(-1001234567890i64));
        assert_eq!(v["parse_mode"], json!("MarkdownV2"));
        assert_eq!(v["link_preview_options"], json!({"is_disabled": true}));
        assert_eq!(v["disable_notification"], json!(false));
        assert!(v.get("message_id").is_none());
        assert!(v.get("disable_web_page_preview").is_none());
    }

    #[test]
    fn parse_mode_none_omits_the_field() {
        let mut r = base();
        r.parse_mode = ParseMode::None;
        assert!(r.to_json().get("parse_mode").is_none());
    }

    #[test]
    fn username_chat_id_stays_a_string() {
        let mut r = base();
        r.chat_id = "@my_channel".parse().unwrap();
        assert_eq!(r.to_json()["chat_id"], json!("@my_channel"));
    }

    #[test]
    fn thread_id_is_included_only_on_send() {
        let mut r = base();
        r.message_thread_id = Some("7".parse().unwrap());
        assert_eq!(r.to_json()["message_thread_id"], json!(7));
        assert_eq!(r.endpoint(), "sendMessage");

        r.edit_message_id = Some("42".parse().unwrap());
        let v = r.to_json();
        assert_eq!(r.endpoint(), "editMessageText");
        assert_eq!(v["message_id"], json!(42));
        assert!(v.get("message_thread_id").is_none());
        assert!(v.get("disable_notification").is_none());
    }
}
