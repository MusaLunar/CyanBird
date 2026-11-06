# Provider compatibility

| Provider | UI fixture | Windows discovery | Export verification |
|---|---:|---:|---:|
| Claude Code | ready | scaffolded | fixture-only |
| Antigravity | ready | scaffolded | fixture-only |
| Codex / ChatGPT | ready | scaffolded | fixture-only |
| Zcode | ready | scaffolded | fixture-only |
| DeepSeek Harness | ready | scaffolded | fixture-only |

Real path and format bindings are intentionally isolated behind `ProviderAdapter`; the current build does not claim live provider writes.
