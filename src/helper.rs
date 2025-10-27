use colored::*;

/// 辅助工具函数
pub struct Helper;

impl Helper {
    /// 输出彩色文本到终端
    pub fn echo(text: &str, color: &str) {
        let colored_text = match color {
            "red" => text.red(),
            "green" => text.green(),
            "yellow" => text.yellow(),
            "blue" => text.blue(),
            "magenta" => text.magenta(),
            "cyan" => text.cyan(),
            _ => text.white(),
        };
        println!("{}", colored_text);
    }

    /// 计算 g_tk (GTK)
    /// 用于QQ API认证
    pub fn gtk(skey: &str) -> i64 {
        let mut hash: i64 = 5381;
        for ch in skey.chars() {
            hash += (hash << 5 & 2147483647) + (ch as i64) & 2147483647;
            hash &= 2147483647;
        }
        hash & 2147483647
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gtk() {
        let skey = "test_key";
        let result = Helper::gtk(skey);
        assert!(result > 0);
    }
}

