use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::helper::Helper;

/// 数据库操作类
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 创建新的数据库实例
    pub fn new(db_file: &str) -> Result<Self> {
        let conn = Connection::open(db_file)
            .with_context(|| format!("无法打开数据库: {}", db_file))?;

        let db = Database { conn };
        db.create_table()?;
        Ok(db)
    }

    /// 创建消息表
    fn create_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_uin INTEGER NOT NULL,
                to_uin INTEGER NOT NULL,
                from_uid TEXT NOT NULL,
                to_uid TEXT NOT NULL,
                msg_seq INTEGER NOT NULL,
                msg_uid TEXT NOT NULL,
                random INTEGER NOT NULL,
                client_seq INTEGER NOT NULL,
                msg_time INTEGER NOT NULL,
                body TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(msg_seq)
            )",
            [],
        )?;
        
        // 创建索引以提高查询性能
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_msg_time ON messages(msg_time DESC)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_from_uin ON messages(from_uin)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_to_uin ON messages(to_uin)",
            [],
        )?;
        
        Ok(())
    }

    /// 保存单条消息
    pub fn save_message(&self, message: &Value) -> Result<bool> {
        let routing_head = message
            .get("routing_head")
            .with_context(|| "消息缺少routing_head")?;
        let content_head = message
            .get("content_head")
            .with_context(|| "消息缺少content_head")?;

        // 辅助函数：解析可能是字符串或数字的值
        let parse_to_i64 = |v: &Value| -> i64 {
            match v {
                Value::Number(n) => n.as_i64().unwrap_or(0),
                Value::String(s) => s.parse::<i64>().unwrap_or(0),
                _ => 0,
            }
        };

        let from_uin = routing_head.get("from_uin").map(parse_to_i64).unwrap_or(0);
        let to_uin = routing_head.get("to_uin").map(parse_to_i64).unwrap_or(0);
        let from_uid = routing_head
            .get("from_uid")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let to_uid = routing_head
            .get("to_uid")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // msg_seq可能为0，也需要保存
        let msg_seq = content_head
            .get("msg_seq")
            .map(parse_to_i64)
            .unwrap_or_else(|| {
                // 如果没有msg_seq，使用msg_time作为备用标识
                content_head.get("msg_time").map(parse_to_i64).unwrap_or(0)
            });
        let msg_uid = content_head
            .get("msg_uid")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let random = content_head.get("random").map(parse_to_i64).unwrap_or(0);
        let client_seq = content_head.get("client_seq").map(parse_to_i64).unwrap_or(0);
        let msg_time = content_head.get("msg_time").map(parse_to_i64).unwrap_or(0);

        let body = message.get("body").with_context(|| "消息缺少body")?;
        let body_str = serde_json::to_string(body)?;

        // 使用INSERT OR REPLACE来处理重复消息
        // 如果msg_seq已存在，则更新；否则插入新记录
        self.conn.execute(
            "INSERT INTO messages (from_uin, to_uin, from_uid, to_uid, msg_seq, msg_uid, 
             random, client_seq, msg_time, body) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(msg_seq) DO UPDATE SET
             from_uin=excluded.from_uin,
             to_uin=excluded.to_uin,
             from_uid=excluded.from_uid,
             to_uid=excluded.to_uid,
             msg_uid=excluded.msg_uid,
             random=excluded.random,
             client_seq=excluded.client_seq,
             msg_time=excluded.msg_time,
             body=excluded.body",
            params![
                from_uin, to_uin, from_uid, to_uid, msg_seq, msg_uid, random, client_seq,
                msg_time, body_str
            ],
        )?;

        Ok(true)
    }

    /// 批量保存消息
    pub fn save_messages(&self, messages: &[Value]) -> Result<(usize, usize)> {
        let mut success = 0;
        let mut failed = 0;

        let tx = self.conn.unchecked_transaction()?;

        for message in messages {
            match self.save_message(message) {
                Ok(_) => success += 1,
                Err(e) => {
                    Helper::echo(
                        &format!("保存消息失败: {}", e),
                        "red",
                    );
                    failed += 1;
                }
            }
        }

        tx.commit()?;

        Ok((success, failed))
    }

    /// 检查消息是否存在
    #[allow(dead_code)]
    pub fn message_exists(&self, msg_seq: i64) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE msg_seq = ?1",
            params![msg_seq],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 获取消息总数
    #[allow(dead_code)]
    pub fn get_message_count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
        Ok(count)
    }

    /// 获取所有消息（按msg_time降序排序）
    #[allow(dead_code)]
    pub fn get_all_messages(&self, limit: i64, offset: i64) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_uin, to_uin, from_uid, to_uid, msg_seq, msg_uid, 
             random, client_seq, msg_time, body, created_at 
             FROM messages 
             ORDER BY msg_time DESC, id DESC 
             LIMIT ?1 OFFSET ?2"
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| {
            let body_str: String = row.get(10)?;
            let body: Value = serde_json::from_str(&body_str).unwrap_or(serde_json::json!([]));
            
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "from_uin": row.get::<_, i64>(1)?,
                "to_uin": row.get::<_, i64>(2)?,
                "from_uid": row.get::<_, String>(3)?,
                "to_uid": row.get::<_, String>(4)?,
                "msg_seq": row.get::<_, i64>(5)?,
                "msg_uid": row.get::<_, String>(6)?,
                "random": row.get::<_, i64>(7)?,
                "client_seq": row.get::<_, i64>(8)?,
                "msg_time": row.get::<_, i64>(9)?,
                "body": body,
                "created_at": row.get::<_, String>(11)?,
            }))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 按时间范围查询消息（按msg_time降序）
    #[allow(dead_code)]
    pub fn get_messages_by_time_range(&self, start_time: i64, end_time: i64) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_uin, to_uin, from_uid, to_uid, msg_seq, msg_uid, 
             random, client_seq, msg_time, body, created_at 
             FROM messages 
             WHERE msg_time BETWEEN ?1 AND ?2 
             ORDER BY msg_time DESC, id DESC"
        )?;

        let rows = stmt.query_map(params![start_time, end_time], |row| {
            let body_str: String = row.get(10)?;
            let body: Value = serde_json::from_str(&body_str).unwrap_or(serde_json::json!([]));
            
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "from_uin": row.get::<_, i64>(1)?,
                "to_uin": row.get::<_, i64>(2)?,
                "from_uid": row.get::<_, String>(3)?,
                "to_uid": row.get::<_, String>(4)?,
                "msg_seq": row.get::<_, i64>(5)?,
                "msg_uid": row.get::<_, String>(6)?,
                "random": row.get::<_, i64>(7)?,
                "client_seq": row.get::<_, i64>(8)?,
                "msg_time": row.get::<_, i64>(9)?,
                "body": body,
                "created_at": row.get::<_, String>(11)?,
            }))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

