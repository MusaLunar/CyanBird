# CyanBird 设计规格

日期：2026-11-06
状态：已获用户批准，待 spec 审阅

## 1. 目标

CyanBird 是一个 Windows-first 的本地桌面应用，用于在 Claude Code、Antigravity、Codex（ChatGPT）、Zcode 和 DeepSeek Harness（DSH）之间迁移与备份 AI agent 会话。

首版以迁移工作台为主流程：发现本机来源 → 选择会话 → 选择目标 → 预览转换 → 创建备份 → 执行迁移 → 校验结果。

产品要求：UI 简洁、启动快、内存占用低、迁移过程可恢复、原始会话不被静默破坏。

## 2. 用户与成功标准

目标用户是同时使用多个 AI coding agent 的开发者。首版成功标准：

- 能自动发现并展示五个 provider 的本机会话目录；
- 能解析至少一组稳定 fixture，映射到统一会话模型；
- 能从任意已支持来源生成目标 provider 的迁移预览；
- 执行迁移前自动生成可恢复快照；
- 迁移完成后提供逐会话状态、校验结果和恢复入口；
- 中英双语切换无需重启；
- Windows 安装包保持轻量，核心操作不依赖联网。

## 3. 非目标

首版不做云端同步、账号体系、团队协作、远程路径挂载、在线模型调用、跨设备冲突合并。

## 4. 技术方案

采用 Tauri + React：

- React/TypeScript：页面、筛选、预览、进度、设置和国际化；
- Rust/Tauri：目录探测、文件读取、解析编排、快照、原子写入、校验和错误回传；
- provider adapter：每个 agent 独立实现 discovery、parse、normalize、export、verify；
- 本地状态：轻量 JSON/SQLite 均可，首版优先 JSON manifest，避免引入不必要的数据库运行时。

目录结构建议：

```text
src/
  app/
  components/
  features/sources/
  features/sessions/
  features/migration/
  features/settings/
  i18n/
  types/
src-tauri/
  src/commands/
  src/core/
  src/providers/
  src/snapshots/
  src/verification/
tests/fixtures/
docs/
```

## 5. 核心数据模型

```ts
interface NormalizedSession {
  id: string;
  title: string;
  source: ProviderId;
  workspace?: string;
  createdAt?: string;
  updatedAt?: string;
  messages: Message[];
  toolCalls: ToolCall[];
  reasoning: ReasoningBlock[];
  attachments: Attachment[];
  metadata: Record<string, unknown>;
}

interface MigrationPlan {
  sourceProvider: ProviderId;
  targetProvider: ProviderId;
  sessionIds: string[];
  targetWorkspace?: string;
  conflictPolicy: 'new-id' | 'overwrite' | 'skip';
  createBackup: boolean;
}
```

内部模型必须保留未知字段到 `metadata`，避免跨 provider 转换时丢失信息。无法表达的字段在预览中标记为 `unsupported`，由用户在执行前确认。

## 6. Provider 适配器

统一接口：

```rust
trait ProviderAdapter {
    fn id(&self) -> ProviderId;
    fn discover(&self) -> Result<Vec<SourceLocation>, ProviderError>;
    fn list_sessions(&self, source: &SourceLocation) -> Result<Vec<SessionSummary>, ProviderError>;
    fn parse(&self, session: &SessionSummary) -> Result<NormalizedSession, ProviderError>;
    fn preview_export(&self, session: &NormalizedSession, target: &TargetContext) -> Result<ExportPreview, ProviderError>;
    fn export(&self, session: &NormalizedSession, target: &TargetContext) -> Result<ExportReceipt, ProviderError>;
    fn verify(&self, receipt: &ExportReceipt) -> Result<VerificationReport, ProviderError>;
}
```

首版 provider：Claude Code、Antigravity、Codex、Zcode、DSH。真实路径与格式通过 Windows fixture、参考项目和本机探测结果绑定；没有证据的路径不写死，显示为待探测状态。

## 7. UI 信息架构

### 7.1 Shell

左侧窄导航：Workspace、Migration、Backups、Settings。顶部显示 CyanBird 品牌、当前语言、扫描按钮和任务状态。

### 7.2 Migration 工作台

- Sources：provider 卡片、状态、路径、session 数量、最近扫描时间；
- Sessions：搜索、来源筛选、workspace 筛选、多选、更新时间和解析状态；
- Migration plan：目标 provider、目标 workspace、冲突策略、备份开关、转换摘要；
- Preview drawer：消息数、tool call 数、reasoning 数、附件、未支持字段和目标文件布局；
- Progress view：逐会话状态、当前阶段、错误详情、恢复入口。

### 7.3 Backups

展示迁移生成的快照、manifest、大小、时间、来源和恢复按钮。首版不做独立复杂备份编排，所有备份都从迁移记录进入。

### 7.4 Settings

语言、中英切换、默认备份目录、扫描范围、是否自动发现、主题模式。

## 8. 迁移事务与错误处理

每次迁移按以下顺序执行：

1. 读取并校验源会话；
2. 写入带时间戳的 backup snapshot 和 manifest；
3. 在目标目录创建临时写入区；
4. 导出到临时区；
5. 运行目标 provider verify；
6. 校验通过后执行原子替换或提交；
7. 写入 migration report。

失败规则：

- 单个 session 失败不隐藏其他成功结果；
- 源文件保持不变；
- 临时目录和快照保留，便于诊断与恢复；
- 错误包含 provider、路径、阶段、原始错误和建议动作；
- 冲突默认使用 `new-id`，覆盖操作必须显式选择。

## 9. 国际化与视觉

首版提供 `zh-CN` 和 `en-US`。UI 文案可切换；provider 名称、路径、JSONL、tool call 等技术字段保持原文。

视觉方向：冷青色品牌色、浅色背景、细边框、低阴影、紧凑但舒适的间距；深色模式采用同一套语义 token。交互反馈优先使用状态标签、进度和局部 toast，避免重型弹窗。

## 10. 测试策略

- provider fixture：发现、解析、normalize、export、verify；
- 往返测试：parse → normalize → export → parse；
- 迁移计划：冲突策略、空选择、部分失败；
- 快照：manifest 完整性、恢复路径、原子提交；
- Windows：路径包含空格、Unicode、锁文件、权限不足；
- UI：工作台主要状态、空态、错误态、进度态、双语切换。

## 11. 交付分阶段

### Phase 1：可视化工作台骨架

完成 Tauri/React shell、三栏迁移工作台、双语、fixture 驱动的假数据预览和进度状态。

### Phase 2：本地发现与统一模型

完成 Windows 路径探测、provider registry、统一模型、至少两个 provider 的真实解析链。

### Phase 3：迁移与恢复

完成五个 provider adapter 的可用链路、快照、导出、验证、恢复和报告。

### Phase 4：质量与发布

完成错误路径、性能检查、安装包、用户文档和兼容性清单。

## 12. 风险与决策

- 不同 agent 的格式可能持续变化，因此 adapter 必须隔离，且保留未知字段；
- DSH 可能同时存在 workspace/session 两层状态，先以 reference fixture 验证，再确定映射；
- 部分 agent 可能需要 CLI 或环境变量才能验证导入，verify 必须支持“文件级校验”和“可选运行时校验”两级；
- 首版优先可靠迁移和可恢复性，自动清理与高阶合并延后。
