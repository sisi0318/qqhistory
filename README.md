# 此为半成品 仅供参考

# QQ历史消息拉取工具

## 1. 准备cookie.json

使用抓包抓取
https://ntlogin.qq.com/wxmini/login/exchange 的返回值 （微信小程序 腾讯QQ）

将有效的`cookie.json`文件放在根目录
cookie.json的格式示例：
```json
{
  "result": 0,
  "msg": "",
  "req_id": "...",
  "account": "你的QQ号",
  "nickname": "昵称",
  "avatar_url": "头像URL",
  "tickets": [
    {
      "name": "pskey",
      "domain": "myqq.qq.com",
      "ticket": "票据内容",
      "expire_at": 1234567890 // 通常可用三天
    }
  ]
}
```

## 2. 使用

cargo build --relsese 

qqhistory.exe --uin=uin

## 当前仅支持
文字 / 图片（nt） / 回复 / 语音（nt） /视频（nt）
标记的nt均为 nt_rich_media