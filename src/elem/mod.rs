pub mod parser;
pub mod text_elem;
pub mod image_elem;
pub mod video_elem;
pub mod voice_elem;
pub mod common_elem;
pub mod reply_elem;

use serde_json::Value;

/// 解析器接口trait
pub trait ParserInterface {
    fn parse(&self, data: &Value, full_elem: Option<&Value>) -> Option<Value>;
}

