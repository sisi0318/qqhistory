use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

/// Protobuf Wire类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireType {
    Varint = 0,
    Bit64 = 1,
    LengthDelimited = 2,
    Bit32 = 5,
}

impl WireType {
    fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(WireType::Varint),
            1 => Ok(WireType::Bit64),
            2 => Ok(WireType::LengthDelimited),
            5 => Ok(WireType::Bit32),
            _ => Err(anyhow!("不支持的wire type: {}", value)),
        }
    }
}

/// Protobuf反序列化器
pub struct Protobuf;

impl Protobuf {
    /// 反序列化Protobuf二进制数据为HashMap
    pub fn deserialize(data: &[u8]) -> Result<HashMap<u32, Value>> {
        let mut result = HashMap::new();
        let mut offset = 0;

        while offset < data.len() {
            let (field_number, wire_type, new_offset) = Self::decode_tag(data, offset)?;
            offset = new_offset;

            let (value, new_offset) = Self::decode_value(data, offset, wire_type)?;
            offset = new_offset;

            // 处理重复字段
            result.entry(field_number)
                .and_modify(|existing| {
                    if let Value::Array(arr) = existing {
                        arr.push(value.clone());
                    } else {
                        *existing = json!([existing.clone(), value.clone()]);
                    }
                })
                .or_insert(value);
        }

        Ok(result)
    }

    /// 解码标签（tag）
    fn decode_tag(data: &[u8], offset: usize) -> Result<(u32, WireType, usize)> {
        let (tag, new_offset) = Self::decode_varint(data, offset)?;
        let field_number = (tag >> 3) as u32;
        let wire_type = WireType::from_u8((tag & 0x07) as u8)?;
        Ok((field_number, wire_type, new_offset))
    }

    /// 解码变长整数（varint）
    fn decode_varint(data: &[u8], mut offset: usize) -> Result<(u64, usize)> {
        let mut value: u64 = 0;
        let mut shift = 0;

        loop {
            if offset >= data.len() {
                return Err(anyhow!("读取varint时数据意外结束"));
            }

            let byte = data[offset];
            offset += 1;

            value |= ((byte & 0x7F) as u64) << shift;
            shift += 7;

            if (byte & 0x80) == 0 {
                break;
            }
        }

        Ok((value, offset))
    }

    /// 解码64位值
    fn decode_64bit(data: &[u8], offset: usize) -> Result<(Value, usize)> {
        if offset + 8 > data.len() {
            return Err(anyhow!("64位值数据不足"));
        }

        let bytes = &data[offset..offset + 8];
        let value = u64::from_le_bytes(bytes.try_into().unwrap());
        Ok((json!(value), offset + 8))
    }

    /// 解码32位值
    fn decode_32bit(data: &[u8], offset: usize) -> Result<(Value, usize)> {
        if offset + 4 > data.len() {
            return Err(anyhow!("32位值数据不足"));
        }

        let bytes = &data[offset..offset + 4];
        let value = u32::from_le_bytes(bytes.try_into().unwrap());
        Ok((json!(value), offset + 4))
    }

    /// 解码长度限定值（length-delimited）
    fn decode_length_delimited(data: &[u8], offset: usize) -> Result<(Value, usize)> {
        let (length, new_offset) = Self::decode_varint(data, offset)?;
        let length = length as usize;

        if new_offset + length > data.len() {
            return Err(anyhow!("长度限定值数据不足"));
        }

        let bytes = &data[new_offset..new_offset + length];
        let new_offset = new_offset + length;

        // 尝试递归解析嵌套消息
        match Self::deserialize(bytes) {
            Ok(nested) => {
                // 成功解析为嵌套消息
                let nested_json: Value = serde_json::to_value(nested)?;
                Ok((nested_json, new_offset))
            }
            Err(_) => {
                // 无法解析为嵌套消息，尝试作为字符串
                if let Ok(s) = std::str::from_utf8(bytes) {
                    Ok((json!(s), new_offset))
                } else {
                    // 作为base64编码的字节数组
                    let encoded = general_purpose::STANDARD.encode(bytes);
                    Ok((json!(encoded), new_offset))
                }
            }
        }
    }

    /// 解码值
    fn decode_value(data: &[u8], offset: usize, wire_type: WireType) -> Result<(Value, usize)> {
        match wire_type {
            WireType::Varint => {
                let (value, new_offset) = Self::decode_varint(data, offset)?;
                Ok((json!(value), new_offset))
            }
            WireType::Bit64 => Self::decode_64bit(data, offset),
            WireType::LengthDelimited => Self::decode_length_delimited(data, offset),
            WireType::Bit32 => Self::decode_32bit(data, offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_varint() {
        let data = vec![0x96, 0x01]; // 150
        let (value, offset) = Protobuf::decode_varint(&data, 0).unwrap();
        assert_eq!(value, 150);
        assert_eq!(offset, 2);
    }
}

