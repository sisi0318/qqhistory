use serde_json::{json, Value};
use crate::elem::ParserInterface;
use base64::{Engine as _, engine::general_purpose};

/// 文本消息解析器
pub struct TextElem;

impl ParserInterface for TextElem {
    fn parse(&self, data: &Value, _full_elem: Option<&Value>) -> Option<Value> {
        let str_value = data.get("str")?.as_str()?;
        let content = general_purpose::STANDARD.decode(str_value).ok()?;
        let content_str = String::from_utf8(content).ok()?;

        if content_str.is_empty() {
            return None;
        }

        Some(json!({
            "type": "text",
            "content": content_str
        }))
    }
}

