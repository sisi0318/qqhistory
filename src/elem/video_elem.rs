use serde_json::{json, Value};
use crate::elem::ParserInterface;
use crate::protobuf::Protobuf;
use base64::{Engine as _, engine::general_purpose};

/// 视频消息解析器
pub struct VideoElem;

impl ParserInterface for VideoElem {
    fn parse(&self, data: &Value, _full_elem: Option<&Value>) -> Option<Value> {
        let pb_elem = data.get("bytes_pb_elem")?.as_str()?;
        let pb_bytes = general_purpose::STANDARD.decode(pb_elem).ok()?;
        let video_body = Protobuf::deserialize(&pb_bytes).ok()?;
        let video_body = serde_json::to_value(&video_body).ok()?;

        let items = video_body.get("1")?.as_array()?;
        if items.is_empty() {
            return None;
        }

        let video_info = Self::extract_video_info(items)?;
        let thumb_info = Self::extract_thumb_info(items);

        Some(json!({
            "type": "video",
            "video": {
                "url": "",
                "duration": video_info.get("file_info")?.get("duration").unwrap_or(&json!(0)),
                "thumb_url": thumb_info.as_ref().and_then(|v| v.get("url")).unwrap_or(&json!("")),
                "richmedia": {
                    "video": video_info,
                    "thumb": thumb_info,
                }
            }
        }))
    }
}

impl VideoElem {
    fn extract_video_info(items: &[Value]) -> Option<Value> {
        let video_item = items.get(0)?;
        let video_data = video_item.get("1")?;
        let file_info = video_data.get("1")?;

        let default_empty = json!("");
        let default_zero = json!(0);
        
        let file_uuid = video_data.get("2").unwrap_or(&default_empty);

        Some(json!({
            "file_info": {
                "size": file_info.get("1").unwrap_or(&default_zero),
                "md5": file_info.get("2").unwrap_or(&default_empty),
                "sha1": file_info.get("3").unwrap_or(&default_empty),
                "file_name": file_info.get("4").unwrap_or(&default_empty),
                "width": file_info.get("6").unwrap_or(&default_zero),
                "height": file_info.get("7").unwrap_or(&default_zero),
                "duration": file_info.get("8").unwrap_or(&default_zero),
                "format": file_info.get("9").unwrap_or(&default_zero),
            },
            "file_uuid": file_uuid,
            "upload_time": video_data.get("4").unwrap_or(&default_zero),
        }))
    }

    fn extract_thumb_info(items: &[Value]) -> Option<Value> {
        let thumb_item = items.get(1)?;
        let thumb_data = thumb_item.get("1")?;
        let file_info = thumb_data.get("1")?;

        let default_empty = json!("");
        let default_zero = json!(0);
        
        let file_uuid = thumb_data.get("2").unwrap_or(&default_empty);

        Some(json!({
            "file_info": {
                "size": file_info.get("1").unwrap_or(&default_zero),
                "md5": file_info.get("2").unwrap_or(&default_empty),
                "sha1": file_info.get("3").unwrap_or(&default_empty),
                "file_name": file_info.get("4").unwrap_or(&default_empty),
                "width": file_info.get("6").unwrap_or(&default_zero),
                "height": file_info.get("7").unwrap_or(&default_zero),
            },
            "file_uuid": file_uuid,
            "url": "",
            "upload_time": thumb_data.get("4").unwrap_or(&default_zero),
        }))
    }
}

