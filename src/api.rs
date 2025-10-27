use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use crate::cookie::LoginInfo;
use base64::{Engine as _, engine::general_purpose};

const API_URL: &str = "https://myqq.qq.com/qunng/http2rpc/gotrpc/v1/";

/// API响应结构
#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub retcode: i32,
    pub data: Option<Value>,
}

/// API客户端
pub struct Api {
    client: Client,
    login_info: LoginInfo,
}

impl Api {
    /// 创建新的API客户端
    pub fn new(login_info: LoginInfo) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        Ok(Api {
            client,
            login_info,
        })
    }

    /// 构建x-oidb头
    fn build_x_oidb(&self, cmd: &str) -> Result<String> {
        let oidb_map: HashMap<&str, &str> = [
            ("trpc.msg.nt_register_proxy.RegisterProxy", "0x92cb_1"),
            ("trpc.relation.friendlist/GetFriendList", "0xfd4_2"),
            ("trpc.msg.nt_register_proxy.RegisterProxy/SsoGetRoamMsg", "0x913f_2"),
        ]
        .iter()
        .cloned()
        .collect();

        let oidb = oidb_map
            .get(cmd)
            .with_context(|| format!("未知的命令: {}", cmd))?;

        let parts: Vec<&str> = oidb.split('_').collect();
        let obj = json!({
            "uint32_command": parts[0],
            "uint32_service_type": parts[1],
        });

        Ok(serde_json::to_string(&obj)?)
    }

    /// 发送API请求
    async fn request(&self, cmd: &str, data: Value) -> Result<ApiResponse> {
        let x_oidb = self.build_x_oidb(cmd)?;
        let api_url = format!("{}{}?g_tk={}", API_URL, cmd, self.login_info.g_tk);

        let response = self
            .client
            .post(&api_url)
            .header("host", "myqq.qq.com")
            .header("x-oidb", x_oidb)
            .header("cookie", &self.login_info.cookie)
            .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 18_6_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.64(0x1800402b) NetType/WIFI Language/zh_CN")
            .json(&data)
            .send()
            .await?;

        let api_response: ApiResponse = response.json().await?;
        Ok(api_response)
    }

    /// 获取好友列表
    pub async fn get_friend_list(&self, num: u32) -> Result<Value> {
        let post = json!({
            "bytes_req_paging_cookie": "",
            "uint32_paging_get_num": num,
            "uint32_req_friend_group_info": 1,
            "uint64_friendlist_current_large_seq": "47",
            "rpt_uint32_sns_typelist": [13584],
            "uint64_friendlist_update_seq": "47",
            "rpt_msg_req_param": [
                {
                    "uint32_busi_id": 1,
                    "bytes_trans_param": "CgtlZ6KcAavsA6zsAw=="
                }
            ]
        });

        let response = self
            .request("trpc.relation.friendlist/GetFriendList", post)
            .await?;

        if response.retcode == 0 {
            Ok(response.data.unwrap_or(json!({})))
        } else {
            Err(anyhow::anyhow!(
                "获取好友列表失败: retcode={}",
                response.retcode
            ))
        }
    }

    /// 获取离线消息
    pub async fn sso_get_offline_msg(
        &self,
        request_optional: u32,
        sync_cookie: &str,
    ) -> Result<Value> {
        let post = json!({
            "request_optional": request_optional,
            "seq": 0,
            "sync_cookie": sync_cookie,
            "need_wechat_foucus": true
        });

        let response = self
            .request("trpc.msg.nt_register_proxy.RegisterProxy", post)
            .await?;

        if response.retcode == 0 {
            Ok(response.data.unwrap_or(json!({})))
        } else {
            Err(anyhow::anyhow!(
                "获取离线消息失败: retcode={}",
                response.retcode
            ))
        }
    }

    /// 获取漫游消息
    pub async fn sso_get_roam_msg(
        &self,
        uid: &str,
        msg_time: i64,
        random: i64,
        max_cnt: u32,
        order: u32,
    ) -> Result<Value> {
        let post = json!({
            "peer_uid": uid,
            "msg_time": msg_time,
            "random": random,
            "max_cnt": max_cnt,
            "order": order
        });

        let response = self
            .request(
                "trpc.msg.nt_register_proxy.RegisterProxy/SsoGetRoamMsg",
                post,
            )
            .await?;

        if response.retcode == 0 {
            Ok(response.data.unwrap_or(json!({})))
        } else {
            Err(anyhow::anyhow!(
                "获取漫游消息失败: retcode={}",
                response.retcode
            ))
        }
    }

    /// 保存UID映射
    pub fn save_uid(data: &Value) -> Result<HashMap<String, String>> {
        let mut uids: HashMap<String, String> = if std::path::Path::new("uids.json").exists() {
            let content = std::fs::read_to_string("uids.json")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // 处理群消息列表
        if let Some(group_msg_list) = data.get("group_msg_list").and_then(|v| v.as_array()) {
            for group_msg in group_msg_list {
                if let Some(msgs) = group_msg.get("msg").and_then(|v| v.as_array()) {
                    for msg in msgs {
                        Self::extract_uid(&msg, &mut uids);
                    }
                }
            }
        }

        // 处理私聊消息列表
        if let Some(c2c_msg_list) = data.get("c2c_msg_list").and_then(|v| v.as_array()) {
            for c2c_msg in c2c_msg_list {
                if let Some(msgs) = c2c_msg.get("msgs").and_then(|v| v.as_array()) {
                    for msg in msgs {
                        Self::extract_uid(&msg, &mut uids);
                    }
                }
            }
        }

        // 保存到文件
        let json_str = serde_json::to_string_pretty(&uids)?;
        std::fs::write("uids.json", json_str)?;

        Ok(uids)
    }

    /// 从消息中提取UID
    fn extract_uid(msg: &Value, uids: &mut HashMap<String, String>) {
        if let Some(routing_head) = msg.get("routing_head") {
            // 辅助函数：解析可能是字符串或数字的值
            let parse_to_u64 = |v: &Value| -> u64 {
                match v {
                    Value::Number(n) => n.as_u64().unwrap_or(0),
                    Value::String(s) => s.parse::<u64>().unwrap_or(0),
                    _ => 0,
                }
            };

            let from_uin = routing_head
                .get("from_uin")
                .map(parse_to_u64)
                .unwrap_or(0);
            let from_uid = routing_head
                .get("from_uid")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if from_uin > 0 && !from_uid.is_empty() {
                // 检查是否是简单的base64编码的uin
                if let Ok(decoded) = general_purpose::STANDARD.decode(from_uid) {
                    if decoded == from_uin.to_string().as_bytes() {
                        return; // 跳过简单编码
                    }
                }
                uids.insert(from_uin.to_string(), from_uid.to_string());
            }
        }
    }
}

