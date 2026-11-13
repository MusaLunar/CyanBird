# CyanBird v0.9.0-beta 版本说明 (Release Notes)

**发布日期**：2026-11-13  
**适用平台**：Windows x64（依赖 WebView2 运行时）  
**构建产物**：
- 安装包：`CyanBird_0.9.0-beta_x64-setup.exe`
- 绿色可执行文件：`cyanbird.exe`

---

## 版本概览

`v0.9.0-beta` 是一次面向正式发布体验的桌面工作台里程碑更新：一方面在底层彻底修复 **Codex → Antigravity** 迁入后用户主动消息在客户端界面中隐身的关键协议缺陷；另一方面重构 **迁移工作台** 与 **备份中心** 视觉与交互布局，去除测试与实验字样，带来更清晰、稳定的会话备份、校验与跨 Agent 迁移体验。

---

## 重点改进与修复

### 1. 修复 Codex 导入 Antigravity 后用户主动消息不显示的问题
- **问题说明**：此前向 Antigravity `steps` 表写入 `USER_INPUT`（`step_type = 14`）时，仅填充了 protobuf `field 2`（`user_response`），未同步写入 Antigravity Desktop 实际渲染气泡所需的 `field 3`（`items` 中的文本项），导致迁入对话中仅见助手回复而看不见用户提问。
- **修复内容**：
  - `antigravity.rs` 为每条用户消息同时编码 `user_response`（`field 2`）与 `items`（`field 3` -> `TextOrScopeItem.text`）。
  - `codex.rs` 同步强化用户消息解析：支持逐块剔除 `<environment_context>` 等注入内容而不误伤混合块中的真实提问，并对仅以 `event_msg` `user_message` 记录的用户消息做补全与去重。
  - 通过原生 `language_server.exe`（Antigravity Desktop 2.29.0 / `language_server 1.11.0`）双次重启 RPC 校验 `GetCascadeTrajectorySteps`，确认 `userInput.userResponse` 与 `userInput.items` 均完整可读。

### 2. 优化备份中心界面，消除信息杂乱
- **左列表 + 右详情双栏架构**：左侧展示精简备份卡片，右侧呈现选中备份的完整档案（备份 ID、总大小、本地存储路径、包含会话与消息数、按文件列出的字节数与 `SHA-256` 校验值）。
- **分类筛选与搜索**：新增 **全部 / 会话备份 / 索引快照** 标签页及即时搜索框，快速区分人工备份与迁移时自动生成的目标索引快照。
- **顶部工具条与原地导入**：将“打开备份目录”入口收拢至顶部路径栏；点击 **导入 Agent** 后直接在右侧面板原地展开恢复流程，列表不再上下跳动。

### 3. 全新工业风桌面工作台与发布级细节打磨
- 三栏式迁移工作台（来源选择 → 会话列表 → 四步迁移计划与执行轨），支持中英双语、浅色/深色模式、无障碍焦点可见性与 `prefers-reduced-motion` 减弱动效适配。
- 移除发布界面中的内部开发调试字样。
- 统一应用程序与安装包的透明背景黑色人物图标。

---

## 升级与迁移使用提醒

1. **已迁入 Antigravity 的历史会话**：此前版本写入的数据库不会自动补齐 `userInput.items`。如需找回缺失显示的用户发言，请关闭 Antigravity 客户端后，使用 `v0.9.0-beta` 重新迁入该对话（或直接在**备份中心**选中原备份再次**导入 Agent**）。
2. **写入 Antigravity / Zcode 前的状态要求**：
   - 导入 **Antigravity Desktop** 前必须**完全退出** Antigravity 客户端（确认托盘/后台进程已结束），并确保该客户端此前至少启动过一次以初始化本地存储。
   - 导入 **Zcode** 前目标 SQLite 数据库须已初始化且架构匹配。
3. **数据安全保证**：迁移与恢复操作始终基于经过 `SHA-256` 校验的备份快照生成全新会话 ID，不修改原 Agent 历史日志，也不会覆盖已有目标对话。

---

## 验证摘要

- Rust 后端单元测试：`cargo test`（25 passed / 0 failed）
- Antigravity 原生二进制兼容性测试：`scripts/verify-antigravity-native.py`（双次重启 RPC 校验通过，含 `userInput.items` 断言）
- 前端单元测试：`npm test -- --run`（4 test files / 14 passed）
- 生产构建与 Windows NSIS 打包：`npm run build`、`npx tauri build --bundles nsis`（构建及启动存活验证通过）
