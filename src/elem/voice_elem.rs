use serde_json::{json, Value};
use crate::elem::ParserInterface;
use crate::protobuf::Protobuf;
use base64::{Engine as _, engine::general_purpose};

/// 语音消息解析器
pub struct VoiceElem;

impl ParserInterface for VoiceElem {
    fn parse(&self, data: &Value, _full_elem: Option<&Value>) -> Option<Value> {
        let pb_elem = data.get("bytes_pb_elem")?.as_str()?;
        let pb_bytes = general_purpose::STANDARD.decode(pb_elem).ok()?;
        let voice_body = Protobuf::deserialize(&pb_bytes).ok()?;
        let voice_body = serde_json::to_value(&voice_body).ok()?;

        let file_info = Self::extract_file_info(&voice_body)?;

        Some(json!({
            "type": "voice",
            "voice": {
                "url": "",
                "duration": file_info.get("duration").unwrap_or(&json!(0)),
                "richmedia": {
                    "file_info": file_info,
                    "file_uuid": voice_body.get("1")?.get("1")?.get("2").unwrap_or(&json!("")),
                    "download_info": json!({})
                },
                "upload_time": voice_body.get("1")?.get("1")?.get("4").unwrap_or(&json!("")),
            }
        }))
    }
}

impl VoiceElem {
    fn extract_file_info(voice_body: &Value) -> Option<Value> {
        let file_info = voice_body.get("1")?.get("1")?.get("1")?;

        Some(json!({
            "size": file_info.get("1").unwrap_or(&json!(0)),
            "md5": file_info.get("2").unwrap_or(&json!("")),
            "sha1": file_info.get("3").unwrap_or(&json!("")),
            "file_name": file_info.get("4").unwrap_or(&json!("")),
            "duration": file_info.get("8").unwrap_or(&json!(0)),
        }))
    }
}

