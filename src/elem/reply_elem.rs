use serde_json::{json, Value};
use crate::elem::ParserInterface;
use crate::elem::parser::ElemParser;
use crate::protobuf::Protobuf;
use base64::{Engine as _, engine::general_purpose};

/// 回复消息解析器
pub struct ReplyElem;

impl ParserInterface for ReplyElem {
    fn parse(&self, data: &Value, _full_elem: Option<&Value>) -> Option<Value> {
        // 被回复的消息内容
        let reply_elems = data.get("elems")?.as_array()?.clone();
        let reply_parser = ElemParser::new(reply_elems);
        let reply_content = reply_parser.build();

        // 解析 bytes_pb_reserve 获取回复信息
        let pb_reserve = data.get("bytes_pb_reserve")?.as_str()?;
        let pb_bytes = general_purpose::STANDARD.decode(pb_reserve).ok()?;
        let pb_data = Protobuf::deserialize(&pb_bytes).ok()?;
        let pb_value = serde_json::to_value(&pb_data).ok()?;

        // 提取回复信息
        let reply_seq = pb_value.get("8").and_then(|v| v.as_i64()).unwrap_or(0);
        let reply_to_uid = pb_value.get("6").and_then(|v| v.as_str()).unwrap_or("");
        let reply_from_uid = pb_value.get("7").and_then(|v| v.as_str()).unwrap_or("");

        // 获取当前回复的消息内容（需要从外层的 rich_text.elems 中获取非 src_msg 的 text 元素）
        // 这部分在 parser 中处理

        Some(json!({
            "type": "reply",
            "reply": {
                "reply_to": reply_content,
                "seq": reply_seq,
                "to_uid": reply_to_uid,
                "from_uid": reply_from_uid,
            }
        }))
    }
}


