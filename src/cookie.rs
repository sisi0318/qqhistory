use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use crate::helper::Helper;

/// Cookie票据信息
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ticket {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub ticket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

/// Cookie数据结构
#[derive(Debug, Deserialize, Serialize)]
pub struct Cookie {
    pub result: i32,
    pub msg: String,
    pub req_id: String,
    pub account: String,
    pub nickname: String,
    pub avatar_url: String,
    pub tickets: Vec<Ticket>,
}

/// 登录信息
#[derive(Debug, Clone)]
pub struct LoginInfo {
    pub uin: String,
    pub nickname: String,
    pub avatar: String,
    pub p_skey: String,
    pub g_tk: i64,
    pub expire_at: i64,
    pub cookie: String,
}

impl Cookie {
    /// 从JSON文件加载Cookie
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("无法读取cookie文件: {}", path))?;
        
        let cookie: Cookie = serde_json::from_str(&content)
            .with_context(|| "解析cookie.json失败")?;
        
        Ok(cookie)
    }

    /// 转换为登录信息
    pub fn to_login_info(&self) -> Result<LoginInfo> {
        // 查找 myqq.qq.com 域的票据
        let ticket = self.tickets
            .iter()
            .find(|t| t.domain.as_deref() == Some("myqq.qq.com"))
            .with_context(|| "未找到 myqq.qq.com 域的票据")?;

        let g_tk = Helper::gtk(&ticket.ticket);
        
        let cookie_str = format!(
            "uin={}; p_uin={}; p_skey={};",
            self.account, self.account, ticket.ticket
        );

        Ok(LoginInfo {
            uin: self.account.clone(),
            nickname: self.nickname.clone(),
            avatar: self.avatar_url.clone(),
            p_skey: ticket.ticket.clone(),
            g_tk,
            expire_at: ticket.expire_at.unwrap_or(0),
            cookie: cookie_str,
        })
    }

    /// 检查登录信息是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(ticket) = self.tickets.iter().find(|t| t.domain.as_deref() == Some("myqq.qq.com")) {
            let now = chrono::Utc::now().timestamp();
            if let Some(expire_at) = ticket.expire_at {
                return now >= expire_at;
            }
        }
        true
    }
}

