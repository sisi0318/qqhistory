use serde_json::{json, Value};
use crate::elem::ParserInterface;
use crate::protobuf::Protobuf;
use base64::{Engine as _, engine::general_purpose};

/// 图片消息解析器
pub struct ImageElem;

impl ParserInterface for ImageElem {
    fn parse(&self, data: &Value, _full_elem: Option<&Value>) -> Option<Value> {
        let pb_elem = data.get("bytes_pb_elem")?.as_str()?;
        let pb_bytes = general_purpose::STANDARD.decode(pb_elem).ok()?;
        let image_body = Protobuf::deserialize(&pb_bytes).ok()?;
        let image_body = serde_json::to_value(&image_body).ok()?;

        let file_info = Self::extract_file_info(&image_body)?;
        let download_info = Self::extract_download_info(&image_body)?;
        let url = Self::build_image_url(&download_info);

        Some(json!({
            "type": "image",
            "image": {
                "url": url,
                "richmedia": {
                    "file_info": file_info,
                    "file_uuid": image_body.get("1")?.get("1")?.get("2").unwrap_or(&json!("")),
                    "download_info": download_info
                },
                "upload_time": image_body.get("1")?.get("1")?.get("4").unwrap_or(&json!("")),
            }
        }))
    }
}

impl ImageElem {
    fn extract_file_info(image_body: &Value) -> Option<Value> {
        let file_info = image_body.get("1")?.get("1")?.get("1")?;

        Some(json!({
            "size": file_info.get("1").unwrap_or(&json!(0)),
            "md5": file_info.get("2").unwrap_or(&json!("")),
            "sha1": file_info.get("3").unwrap_or(&json!("")),
            "file_name": file_info.get("4").unwrap_or(&json!("")),
            "width": file_info.get("6").unwrap_or(&json!(0)),
            "height": file_info.get("7").unwrap_or(&json!(0)),
        }))
    }

    fn extract_download_info(image_body: &Value) -> Option<Value> {
        let download_req = image_body.get("1")?.get("2")?;

        Some(json!({
            "pic_url_ext_info": {
                "original_parameter": download_req.get("2")?.get("1").unwrap_or(&json!(null)),
                "big_parameter": download_req.get("2")?.get("2").unwrap_or(&json!(null)),
                "thumb_parameter": download_req.get("2")?.get("3").unwrap_or(&json!(null)),
            },
            "domain": download_req.get("3").unwrap_or(&json!("")),
            "url_path": download_req.get("1").unwrap_or(&json!("")),
            "rkey": image_body.get("2")?.get("1")?.get("11")?.get("30").unwrap_or(&json!("")),
        }))
    }

    fn build_image_url(download_info: &Value) -> String {
        let domain = download_info.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        let url_path = download_info.get("url_path").and_then(|v| v.as_str()).unwrap_or("");
        let rkey = download_info.get("rkey").and_then(|v| v.as_str()).unwrap_or("");

        format!("https://{}{}{}", domain, url_path, rkey)
    }
}

