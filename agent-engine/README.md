# WaLiOffice Agent Engine

基于 DeepSeek Harness (DSH) 的智能体引擎层，为 WaLiOffice 提供增强的 Agent 能力。

## 架构

```
┌─────────────────────────────────────────────────────┐
│                    前端 (React)                       │
│  Studio.tsx → chatApi.stream / chatApi.dshStream     │
└──────────────────┬──────────────────┬───────────────┘
                   │                  │
          Rust 自研路径        DSH Agent 路径
                   │                  │
┌──────────────────▼──────┐  ┌───────▼───────────────────┐
│   Rust 后端 (axum)      │  │  DSH Agent Engine (Node)  │
│                         │  │                            │
│  • /api/chat/stream     │  │  • HTTP Host (port 3780)   │
│  • /api/agent/dsh/*     │◄─┤  • ReAct Agent Loop        │
│  • 渲染服务 (docx/pptx) │  │  • 并行工具执行 (10并发)   │
│  • 文件存储             │  │  • 子 Agent 委托           │
│  • SQLite DB            │  │  • 结构化上下文压缩        │
│  • web_search           │  │  • 会话分支 (fork)         │
│  • OCR / 文件提取       │  │  • 错误恢复 & 重试         │
└─────────────────────────┘  └────────────────────────────┘
```

## DSH 带来的增强

| 能力 | Rust 自研 | DSH 增强 |
|------|----------|----------|
| 工具执行 | 串行 | **10 并发滚动池** |
| 上下文压缩 | 简单阈值截断 | **结构化压缩 (tokenMeter + surface 替换)** |
| 子 Agent | 不支持 | **前台/后台/可持续委托** |
| 错误恢复 | 无重试 | **request-error waterfall + 自动重试** |
| 取消机制 | 基础 | **AbortSignal 全链路取消** |
| 会话分支 | 不支持 | **fork() 分支** |
| 插件体系 | 硬编码 | **Cordis DI + HMR** |

## 快速开始

### 前置条件

- Node.js >= 18
- npm/pnpm
- DSH CLI 已安装 (`npm install -g @deepseek-ai/dsh`)
- Rust 后端运行中 (端口 3000)

### 安装

```bash
cd agent-engine
npm install
```

### 开发

```bash
# 方式 1：使用 DSH CLI 启动
npm run dev

# 方式 2：使用 tsx 直接运行
npm run dev:ts
```

### 环境变量

在项目根目录 `.env` 中配置：

```env
# LLM 配置
LLM_TEXT_BASE_URL=https://api.deepseek.com/v1
LLM_TEXT_API_KEY=sk-xxx
LLM_TEXT_MODELS=deepseek-chat
LLM_TEXT_MODELS_DEFAULT=deepseek-chat

# DSH Agent Engine
WALIOFFICE_RUST_URL=http://127.0.0.1:3000
```

## 插件清单

### 核心插件

| 插件 | 说明 |
|------|------|
| `dsh-host-http` | HTTP/SSE 接口，让 Rust 后端代理请求到 DSH |
| `dsh-llm-openai-compatible` | OpenAI 兼容 LLM 适配器，多 Key 轮询 |
| `dsh-persistence-sqlite` | SQLite 会话持久化，与 Rust 共享数据库 |

### Office 工具插件

| 插件 | 对应工具 | 说明 |
|------|---------|------|
| `dsh-tool-ppt-plan` | `ppt_plan` | PPT 大纲规划 |
| `dsh-tool-ppt-generate` | `ppt_generate` | PPT 文件生成 |
| `dsh-tool-doc-generate` | `doc_generate` | Word 文档生成 |
| `dsh-tool-md-generate` | `md_generate` | Markdown 文档生成 |
| `dsh-tool-sheet-generate` | `sheet_generate` | Excel 表格生成 |
| `dsh-tool-chart-generate` | `chart_generate` | ECharts 图表生成 |
| `dsh-tool-drawio-generate` | `drawio_generate` | Draw.io 图表生成 |
| `dsh-tool-image-prompt` | `image_prompt` | 图片生成 |
| `dsh-tool-video-generate` | `video_generate` | 视频生成 |
| `dsh-tool-web-search` | `web_search` | 网络搜索 |

## 工作流程

1. 用户在前端发送消息
2. Rust 后端检查 DSH 是否可用
3. 如果 DSH 可用：
   - Rust 将请求代理到 DSH Agent Engine (port 3780)
   - DSH 通过 ReAct 循环决定调用哪些工具
   - 工具通过 HTTP 回调 Rust 后端执行渲染/存储
   - 结果通过 SSE 流式返回前端
4. 如果 DSH 不可用：
   - 回退到 Rust 自研的 agent_loop

## 开发路线

### 短期 (1-2 周)
- [x] DSH profile 配置
- [x] 10 个 Office 工具插件
- [x] HTTP Host 插件
- [x] LLM 适配器
- [x] SQLite 持久化
- [ ] Rust 端代理路由实现
- [ ] 前端 DSH 切换逻辑
- [ ] 集成测试

### 中期 (1-2 月)
- [ ] 子 Agent 委托（复杂任务拆分）
- [ ] 并行工具执行优化
- [ ] 结构化上下文压缩（替换 Rust 端简单阈值）
- [ ] 会话分支 (fork) 支持
- [ ] AbortSignal 全链路取消

### 长期
- [ ] 评估将 DSH 核心能力移植到 Rust
- [ ] 自定义压缩策略
- [ ] 多模型协作
