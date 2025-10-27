use serde_json::Value;
use crate::elem::ParserInterface;
use crate::elem::image_elem::ImageElem;
use crate::elem::video_elem::VideoElem;
use crate::elem::voice_elem::VoiceElem;
use std::collections::HashMap;

/// 业务类型常量
const BUSINESS_TYPE_IMAGE: u32 = 10;
const BUSINESS_TYPE_VOICE: u32 = 12;
const BUSINESS_TYPE_VIDEO: u32 = 11;

/// 通用元素解析器
pub struct CommonElem {
    sub_parsers: HashMap<u32, Box<dyn ParserInterface + Send + Sync>>,
}

impl CommonElem {
    pub fn new() -> Self {
        let mut sub_parsers: HashMap<u32, Box<dyn ParserInterface + Send + Sync>> = HashMap::new();
        sub_parsers.insert(BUSINESS_TYPE_IMAGE, Box::new(ImageElem));
        sub_parsers.insert(BUSINESS_TYPE_VOICE, Box::new(VoiceElem));
        sub_parsers.insert(BUSINESS_TYPE_VIDEO, Box::new(VideoElem));

        CommonElem { sub_parsers }
    }
}

impl ParserInterface for CommonElem {
    fn parse(&self, data: &Value, full_elem: Option<&Value>) -> Option<Value> {
        // 检查 uint32_service_type 是否为 48
        if let Some(service_type) = data.get("uint32_service_type").and_then(|v| v.as_u64()) {
            if service_type == 48 {
                if let Some(business_type) = data.get("uint32_business_type").and_then(|v| v.as_u64()) {
                    let business_type = business_type as u32;
                    if let Some(parser) = self.sub_parsers.get(&business_type) {
                        return parser.parse(data, full_elem);
                    }
                }
            }
        }

        None
    }
}

