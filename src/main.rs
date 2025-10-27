mod helper;
mod cookie;
mod api;
mod protobuf;
mod database;
mod elem;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use crate::helper::Helper;
use crate::cookie::Cookie;
use crate::api::Api;
use crate::database::Database;
use crate::elem::parser::ElemParser;

/// QQ历史消息拉取工具
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// QQ号
    #[arg(short = 'u', long = "uin")]
    uin: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 加载cookie
    let cookie_path = "cookie.json";
    if !std::path::Path::new(cookie_path).exists() {
        Helper::echo("未找到有效的登录信息", "red");
        return Ok(());
    }

    let cookie = Cookie::load_from_file(cookie_path)?;

    if cookie.is_expired() {
        Helper::echo("登录信息已过期，请重新登录", "red");
        return Ok(());
    }

    let login_info = cookie.to_login_info()?;
    
    println!("已加载登录信息，用户 QQ 号：{}", login_info.uin);
    println!("用户昵称：{}", login_info.nickname);
    println!("用户头像：{}", login_info.avatar);
    println!(
        "登录有效期至：{}",
        chrono::DateTime::from_timestamp(login_info.expire_at, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
    );

    // 创建API客户端
    let api = Api::new(login_info)?;

    // 获取好友列表
    Helper::echo("正在获取好友列表...", "cyan");
    let _friend_list = api.get_friend_list(500).await?;

    // 处理uid映射
    if !std::path::Path::new("uids.json").exists() {
        Helper::echo("正在获取UID映射（第1次）...", "cyan");
        let offline_msg = api.sso_get_offline_msg(14, "").await?;
        let uids = Api::save_uid(&offline_msg)?;
        Helper::echo(&format!("第1次获取到 {} 个UID映射", uids.len()), "green");

        Helper::echo("正在获取UID映射（第2次）...", "cyan");
        let offline_msg = api.sso_get_offline_msg(687, "").await?;
        let uids = Api::save_uid(&offline_msg)?;
        Helper::echo(&format!("总共获取到 {} 个UID映射", uids.len()), "green");
    } else {
        Helper::echo("uids.json已存在，跳过UID映射获取", "cyan");
    }

    // 获取uid参数
    let uid = get_uid(&args.uin)?;

    // 创建db目录
    let db_dir = "db";
    if !std::path::Path::new(db_dir).exists() {
        fs::create_dir_all(db_dir)?;
        Helper::echo(&format!("创建数据库目录: {}", db_dir), "cyan");
    }

    // 使用uid作为数据库文件名
    let db_file = format!("{}/{}.db", db_dir, uid);
    let db = Database::new(&db_file)?;
    Helper::echo(&format!("使用数据库文件: {}", db_file), "cyan");

    // 循环拉取历史消息
    let max_rounds = 1000;
    let mut round = 0;
    let mut total_saved = 0;
    let mut random: i64 = 0;
    let mut res_last_time: i64 = 0;

    loop {
        round += 1;
        Helper::echo(
            &format!(
                "开始第 {} 轮拉取 (random={}, res_last_time={})",
                round, random, res_last_time
            ),
            "yellow",
        );

        let msg_time = if res_last_time == 0 {
            chrono::Utc::now().timestamp()
        } else {
            res_last_time
        };

        let roam_msg = api.sso_get_roam_msg(&uid, msg_time, random, 50, 1).await?;

        // 检查是否有更多消息
        let msgs = match roam_msg.get("msg").and_then(|v| v.as_array()) {
            Some(m) if !m.is_empty() => m,
            _ => {
                Helper::echo(&format!("第 {} 轮没有更多消息，结束拉取。", round), "cyan");
                break;
            }
        };

        // 解析消息
        let mut messages = Vec::new();
        for msg in msgs {
            let content_head = msg.get("content_head").context("缺少content_head")?;
            let body = msg.get("body").context("缺少body")?;
            let elems = body
                .get("rich_text")
                .and_then(|rt| rt.get("elems"))
                .and_then(|e| e.as_array())
                .map(|arr| arr.clone())
                .unwrap_or_default();

            let parser = ElemParser::new(elems);
            let arrays = parser.build();

            let routing_head = msg.get("routing_head").context("缺少routing_head")?;

            // 处理可能是字符串的数字字段
            let parse_to_i64 = |v: &Value| -> i64 {
                match v {
                    Value::Number(n) => n.as_i64().unwrap_or(0),
                    Value::String(s) => s.parse::<i64>().unwrap_or(0),
                    _ => 0,
                }
            };

            let msg_seq = content_head.get("nt_msg_seq")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);
            let client_seq = content_head.get("msg_seq")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);
            let random_val = content_head.get("random")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);
            let msg_time = content_head.get("msg_time")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);
            let from_uin = routing_head.get("from_uin")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);
            let to_uin = routing_head.get("to_uin")
                .map(|v| parse_to_i64(v))
                .unwrap_or(0);

            let message = serde_json::json!({
                "content_head": {
                    "msg_uid": content_head.get("msg_uid").and_then(|v| v.as_str()).unwrap_or(""),
                    "random": random_val,
                    "client_seq": client_seq,
                    "msg_time": msg_time,
                    "msg_seq": msg_seq,
                },
                "routing_head": {
                    "from_uin": from_uin,
                    "to_uin": to_uin,
                    "from_uid": routing_head.get("from_uid").and_then(|v| v.as_str()).unwrap_or(""),
                    "to_uid": routing_head.get("to_uid").and_then(|v| v.as_str()).unwrap_or(""),
                },
                "body": arrays
            });

            messages.push(message);
        }

        // 保存到数据库
        let (success, failed) = db.save_messages(&messages)?;
        Helper::echo(
            &format!(
                "第 {} 批消息保存: 成功 {} 条，失败 {} 条",
                round, success, failed
            ),
            "green",
        );
        total_saved += success;

        // 更新参数
        if let Some(r) = roam_msg.get("random").and_then(|v| v.as_i64()) {
            random = r;
        }
        if let Some(t) = roam_msg.get("res_last_time").and_then(|v| v.as_i64()) {
            res_last_time = t;
        }

        // 休眠1秒
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if round >= max_rounds {
            break;
        }
    }

    Helper::echo(
        &format!("拉取完成，总共保存 {} 条消息", total_saved),
        "cyan",
    );

    Ok(())
}

/// 获取UID
fn get_uid(cli_uin: &Option<String>) -> Result<String> {
    if let Some(uin) = cli_uin {
        // 从uids.json查找
        if std::path::Path::new("uids.json").exists() {
            let content = fs::read_to_string("uids.json")?;
            let map: HashMap<String, String> = serde_json::from_str(&content)?;

            if let Some(uid) = map.get(uin) {
                Helper::echo(&format!("通过 uin {} 匹配到 uid: {}", uin, uid), "cyan");
                return Ok(uid.clone());
            } else {
                Helper::echo(&format!("无法在 uids.json 中匹配到 uin: {}", uin), "yellow");
                return prompt_uid();
            }
        } else {
            Helper::echo("当前目录没有 uids.json，无法根据 uin 匹配。", "yellow");
            return prompt_uid();
        }
    }

    Err(anyhow::anyhow!("请提供有效的 uid 参数，使用 --uin=xxxxx 或 -u xxxxx"))
}

/// 提示用户输入UID
fn prompt_uid() -> Result<String> {
    print!("是否手动输入 uid？(y/n): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "y" || input == "yes" {
        print!("请输入 uid: ");
        io::stdout().flush()?;

        let mut uid = String::new();
        io::stdin().read_line(&mut uid)?;
        let uid = uid.trim();

        if uid.is_empty() {
            Helper::echo("未输入 uid，退出。", "red");
            return Err(anyhow::anyhow!("未输入 uid"));
        }

        Ok(uid.to_string())
    } else {
        Helper::echo("未提供可用的 uid，退出。", "red");
        Err(anyhow::anyhow!("未提供可用的 uid"))
    }
}
