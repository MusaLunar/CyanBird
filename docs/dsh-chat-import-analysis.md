# dsh-chat-import 参考分析

参考版本：0.26.4；commit `c1c4f627d03d95a60dd8bade12212150ed8af276`。
本地副本：`.references/dsh-chat-import`（不提交到 CyanBird 仓库）。
许可证：MIT，Copyright (c) 2026 Nwflower、Scarlett。本次以行为研究为主，Rust 代码为独立实现。

## 根因

CyanBird 原有 App 从 fixtures.ts 读取静态来源和示例会话。此前显示的来源数量并不是磁盘扫描结果。lib/tauri.ts 还会在调用失败后返回示例数据，因此无法分辨扫描错误与真实会话。

## 来源规则与参考位置

| 来源 | 实际结构 | 参考文件 |
| --- | --- | --- |
| Claude Code | ~/.claude/projects/<project>/<id>.jsonl；Windows 另有 LOCALAPPDATA/Claude-3p/claude-code-sessions | lib/discovery/claude.mjs |
| Codex | ~/.codex/sessions、archived_sessions；rollout 日志；session_meta 给 id/cwd，session_index.jsonl 给重命名标题 | lib/discovery/jsonl.mjs |
| Antigravity | ~/.gemini/antigravity、antigravity-cli、antigravity-ide；conversations/<id>.db 或 .pb；正文 brain/<id>/.system_generated/logs/transcript.jsonl；标题 annotations/<id>.pbtxt | lib/discovery/gemini.mjs |
| Zcode | ~/.zcode/cli/db/db.sqlite；session/message/part 表；WAL 中可能有新会话 | lib/discovery/sqlite.mjs、lib/sources/zcode.mjs |
| DSH | DSH_HOME/sessions/<workspace>/<id>/session[.vN].jsonl[.zstd]；CLI 默认 ~/.dsh；桌面端可能在 APPDATA/dsh-desktop/harness | lib/discovery/dsh.mjs |

Codex 与 ChatGPT 网页导出不同：后者为 conversations.json，需要用户显式提供文件，不能用 Codex 的路径探测代替。

## 参考项目值得保留的设计

- Registry 是唯一来源清单，默认根、格式探测、scanner 从同一描述符派生。
- 路径只能缩小候选，最后仍由内容签名拒绝不匹配的文件。
- Claude fork 要看末尾 sessionId，子代理不能当主会话；重命名标题优先于首问。
- Codex session_index 的最后有效记录优先，不能用注入的 AGENTS/environment 上下文当标题。
- 日志扫描使用有界头尾窗口；解析全文与列表发现分层。
- SQLite 只读，查询 schema 自适应，缓存指纹包括 WAL/SHM。
- 失败有 warnings；空结果不代表没有安装，也不能回退为演示数据。

## 本轮范围

已接入原生只读发现命令、真实来源数量与列表，删除桌面端示例回退；修正 Windows 路径、zst/zstd 解压、Claude 主会话、Codex id/工作目录/索引标题，以及日期显示。

仍需继续适配：Claude-3p 元数据格式、Antigravity annotation 标题与工作目录、DSH 桌面多根/压缩日志代次去重、大日志有界扫描、完整消息归一化和真实目标格式导出。Zcode 消息/工具统计当前尚未实现。不能将本轮发现接入等同于完整迁移能力。

## 本机验证结果

原生诊断命令（只输出数量，不保存会话正文）：

```powershell
$env:CYANBIRD_SCAN_DIAGNOSTICS='1'
cargo test --manifest-path src-tauri/Cargo.toml local_scan_diagnostics -- --nocapture
```

本次实测：Claude Code 3、Antigravity 43、Codex 92、Zcode 9、DSH 日志条目 25。数量会随本机数据变化；DSH 仍需代次去重，不能视为原生 UI 数量的完整对账。

验证：前端 7 个测试通过；Rust 4 个测试通过；本机只读诊断通过。测试扫描范围限定为 src，避免把下载的第三方 Node 测试误交给 Vitest。
