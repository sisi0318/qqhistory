use serde_json::Value;
use crate::elem::ParserInterface;
use crate::elem::text_elem::TextElem;
use crate::elem::common_elem::CommonElem;
use crate::elem::reply_elem::ReplyElem;
// use base64::{Engine as _, engine::general_purpose};

/// ELEM解析器主类
pub struct ElemParser {
    elems: Vec<Value>,
}

impl ElemParser {
    /// 创建新的解析器
    pub fn new(elems: Vec<Value>) -> Self {
        ElemParser { elems }
    }

    /// 构建消息数组
    pub fn build(&self) -> Vec<Value> {
        let mut arrays = Vec::new();

        let text_parser = TextElem;
        let common_parser = CommonElem::new();
        let reply_parser = ReplyElem;

        // 检查是否是回复消息
        let has_src_msg = self.elems.iter().any(|elem| elem.get("src_msg").is_some());

        if has_src_msg {
            // 处理回复消息
            let mut reply_content = Vec::new();
            let mut reply_to_content = None;
            let mut reply_info = None;

            for elem in &self.elems {
                if let Some(src_msg) = elem.get("src_msg") {
                    // 解析被回复的消息
                    if let Some(parsed) = reply_parser.parse(src_msg, Some(elem)) {
                        reply_info = Some(parsed);
                    }
                    // 获取被回复的消息内容
                    if let Some(elems) = src_msg.get("elems").and_then(|v| v.as_array()) {
                        let parser = ElemParser::new(elems.clone());
                        reply_to_content = Some(parser.build());
                    }
                } else if let Some(text_data) = elem.get("text") {
                    // 当前回复的内容
                    if let Some(parsed) = text_parser.parse(text_data, Some(elem)) {
                        reply_content.push(parsed);
                    }
                }
            }

            // 组合回复消息
            if let Some(mut info) = reply_info {
                if let Some(reply_obj) = info.get_mut("reply") {
                    if let Some(obj) = reply_obj.as_object_mut() {
                        obj.insert("reply_msg".to_string(), serde_json::json!(reply_content));
                        if let Some(to_content) = reply_to_content {
                            obj.insert("reply_to".to_string(), serde_json::json!(to_content));
                        }
                    }
                }
                arrays.push(info);
            }
        } else {
            // 普通消息处理
            for elem in &self.elems {
                let parsed = if let Some(text_data) = elem.get("text") {
                    text_parser.parse(text_data, Some(elem))
                } else if let Some(common_data) = elem.get("common_elem") {
                    common_parser.parse(common_data, Some(elem))
                } else {
                    None
                };

                if let Some(p) = parsed {
                    arrays.push(p);
                }
            }
        }

        arrays
    }
}

