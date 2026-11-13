# CyanBird

面向 Windows 优先的 AI 编程会话发现、预览、备份、导出与跨 Agent 迁移桌面工作台（Tauri 2 + React 19 + Rust）。

> Windows-first Tauri 2 + React 19 + Rust desktop workbench for discovering, inspecting, backing up, exporting, and migrating AI coding agent conversations across **Claude Code**, **Antigravity Desktop**, **Codex**, **Zcode**, and **DeepSeek Harness (DSH V4)**.

---

## 核心特性 / Highlights

- **三栏式迁移工作台（Migration Workbench）**
  - 自动发现本地主流 AI 编程 Agent 的主会话列表（过滤子代理线程与系统注入提示词），展示消息数、工具调用数、工作区路径与最近更新时间。
  - **独立会话预览抽屉**：随时在迁移前只读查看完整对话内容，不改变多选状态。
  - **四步安全流转轨迹**：`选择来源与会话 → 指定目标 Agent 与工作区 → 生成迁移预览并核对转换差异 → 确认执行`，任何参数变动自动失效旧预览，防止误写入。
- **双栏主从式备份中心（Backup Center）**
  - 按 `全部 / 会话备份 / 索引快照` 分类筛选，支持按会话标题、备份 ID 与 Agent 关键字实时过滤。
  - 右侧档案视图集中展示备份目录、包含会话标题与消息统计，以及按文件汇总的字节数与 `SHA-256` 校验码。
  - **支持离线快照直接恢复**：即使原始会话文件已被删除，也可经由完整性校验后直接作为新会话导入任意受支持的目标 Agent。
- **源端零侵入与强制保护（Non-destructive Safety）**
  - 每次迁移前强制生成经过 `SHA-256` 校验的源端完整快照；Zcode 与 Antigravity 写入前同步备份目标索引/数据库（含 SQLite WAL）。
  - 始终分配全新会话 ID 并以原子/事务方式写入，冲突或校验失败自动回滚，绝不覆盖现有对话。
- **高保真 Markdown 导出**
  - 为选中会话导出独立的 `.md` 文档，完整保留对话正文、推理摘要及以超长反引号围栏保护的字面工具调用 JSON 记录。
- **双语与主题支持**
  - 内置 `简体中文` / `English` 一键切换，支持浅色 / 深色主题与自定义无边框标题栏，以及 GitHub Release 更新检查。

---

## 支持矩阵 / Supported Agents

| Agent | 本地会话发现 | 完整备份 (SHA-256) | Markdown 导出 | 原生迁移目标 | 从备份导入恢复 | 说明 / 前置条件 |
|---|:---:|:---:|:---:|:---:|:---:|---|
| **Claude Code** | ✅ | ✅ | ✅ | ✅ | ✅ | 读写原生 JSONL 会话文件 |
| **Codex** | ✅ | ✅ | ✅ | ✅ | ✅ | 支持分页 rollout 窗口还原与 `user_message` 回退解析 |
| **Antigravity Desktop** | ✅ | ✅ | ✅ | ✅ | ✅ | 仅迁入用户/助手对话正文；需先初始化并**完全退出**客户端后执行 |
| **Zcode** | ✅ | ✅ | ✅ | ✅ | ✅ | 需已存在的 SQLite 会话库；事务写入且全库热备份 |
| **DeepSeek Harness (DSH V4)** | ✅ | ✅ | ✅ | ✅ | ✅ | 原生事件流结构写入 |

详细差异与兼容性限制请参阅 [compatibility.md](docs/compatibility.md) 与 [migration.md](docs/migration.md)。

---

## 开发与构建 / Development

桌面构建依赖 Rust（Windows MSVC 工具链）、Visual C++ Build Tools 以及系统 WebView2 运行时。

```powershell
# 安装前端依赖
npm ci

# 启动 Vite 前端或 Tauri 桌面开发模式
npm run dev
npm run tauri dev

# 前端单元测试与类型/打包检查
npm test -- --run
npm run build

# Rust 后端单元测试
cargo test --manifest-path src-tauri/Cargo.toml

# 原生 Antigravity 写入双次重启校验（需本机存在 Antigravity language_server.exe）
python scripts/verify-antigravity-native.py --binary "$env:LOCALAPPDATA/Programs/antigravity/resources/bin/language_server.exe"

# 构建 Windows Release 可执行文件与 NSIS 安装包
npx tauri build --bundles nsis
```

> 仅浏览器模式运行 `npm run dev` 时无法访问本地原生目录；应用会显式反馈原生命令不可用错误，而不会伪造示例会话。
> 更新检查功能通过查询 `MusaLunar/CyanBird` 的公开 GitHub Release 提供版本提示与跳转链接，暂不执行静默自动安装。

---

## 文档导航 / Documentation

- [更新日志（CHANGELOG）](CHANGELOG.md)
- [v0.9.0-beta 版本说明（Release Notes）](docs/release-notes-v0.9.0-beta.md)
- [历史会话迁移指南与策略](docs/migration.md)
- [Antigravity Desktop 历史迁入说明](docs/antigravity-import.md)
- [备份中心与快照导入恢复说明](docs/backup-import.md)
- [当前能力与兼容性边界](docs/compatibility.md)
- [DSH 会话导入分析参考](docs/dsh-chat-import-analysis.md)
- [Agent 图标来源与许可说明](docs/agent-icons.md)
