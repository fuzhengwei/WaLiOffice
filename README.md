# WaLiOffice

> Web 端 AI Agent 智能办公平台 — 通过自然语言对话，一键生成 PPT、文档、表格、流程图、图表、图片和视频。

WaLiOffice 将 AI Agent 能力与办公文档生成深度结合：用户在对话中描述需求，Agent 自动拆解任务、调用工具、生成产物，全程 SSE 流式响应，实时展示思考和生成过程。生产部署只需一个 Rust 二进制（前端静态资源内嵌），开箱即用。

## ✨ 功能特性

- **AI 对话驱动**：基于 ReAct 循环的 Agent 引擎，支持多轮对话、上下文压缩、自动工具编排
- **PPT 生成**：大纲规划 → 内容填充 → 视觉设计，生成可预览的幻灯片
- **文档生成**：支持报告 / PRD / 计划等多种文档类型，可导出 `.docx`
- **表格生成**：结构化数据表格，可导出 `.xlsx`
- **图表生成**：基于数据自动生成可视化图表
- **流程图生成**：输出 draw.io 格式 XML，支持在线编辑
- **图片生成**：调用文生图模型，生成高质量图片
- **视频生成**：调用文生视频模型，远程不可用时本地 `ffmpeg` 兜底合成 MP4
- **联网搜索**：支持 SearXNG / DuckDuckGo 多搜索源，Agent 可主动检索实时信息
- **文件解析**：支持上传文件内容提取与 OCR 识别，作为对话上下文
- **多模型切换**：文本 / 图片 / 视频各自独立配置模型列表，前端设置页可实时切换
- **用户认证**：JWT 认证 + 公众号验证码登录，支持注册和登录验证码
- **单一二进制部署**：Rust + rust-embed，前端嵌入后端，一个二进制即可启动全部服务
- **Docker 一键部署**：多阶段构建，内置国内镜像加速

## 🏗 技术栈

| 层 | 技术 | 说明 |
|---|---|---|
| 后端框架 | Rust + axum + tokio | 高性能异步 Web 框架 |
| 前端框架 | React 18 + TypeScript + Vite | SPA，构建后嵌入 Rust 二进制 |
| 样式 | TailwindCSS | 原子化 CSS |
| 状态管理 | Zustand | 轻量级状态管理 |
| 数据库 | SQLite（嵌入式） | 零配置，可选 MySQL |
| LLM 接口 | OpenAI 兼容 | 支持任何兼容端点，流式 SSE |
| 文档渲染 | docx-rs + rust_xlsxwriter | 纯 Rust 实现，无外部依赖 |
| 认证 | JWT + bcrypt | 无状态认证 |
| 静态资源 | rust-embed | 编译时嵌入前端产物 |
| 部署 | Docker / 单一二进制 | 多阶段构建，镜像 < 50MB |

## 📐 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                    浏览器 / React UI                      │
│  Studio · Login · Files · Settings                       │
└────────────────────────┬────────────────────────────────┘
                         │ HTTP / SSE
┌────────────────────────▼────────────────────────────────┐
│                  Rust Axum Server                         │
│  ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌──────────────┐ │
│  │  Auth   │ │  Routes  │ │  Agent  │ │   Render     │ │
│  │ (JWT)   │ │ (REST+   │ │ (ReAct) │ │ (DOCX/XLSX)  │ │
│  │         │ │  SSE)    │ │         │ │              │ │
│  └─────────┘ └──────────┘ └────┬───┘ └──────────────┘ │
│                               │                         │
│              ┌────────────────┼─────────────┐          │
│              ▼                ▼             ▼          │
│        ┌──────────┐  ┌──────────────┐ ┌──────────┐    │
│        │   LLM    │  │    Tools     │ │  SQLite  │    │
│        │  Client  │  │  Registry    │ │          │    │
│        └──────────┘  └──────┬───────┘ └──────────┘    │
│                             │                          │
│    ┌──────┬──────┬─────────┼────────┬──────┬──────┐  │
│    ▼      ▼      ▼         ▼        ▼      ▼      ▼  │
│   PPT    DOC   Sheet    DrawIO   Image  Video  Search │
└─────────────────────────────────────────────────────────┘
```

## 📁 项目结构

```
WaLiOffice/
├── server/                   # Rust 服务端（~14000 行）
│   └── src/
│       ├── main.rs           # 启动入口
│       ├── config.rs         # 环境变量配置
│       ├── state.rs          # 应用状态
│       ├── error.rs          # 统一错误处理
│       ├── file_extract.rs   # 文件内容提取
│       ├── image_ocr.rs      # OCR 图片识别
│       ├── agent/            # Agent 引擎
│       │   ├── mod.rs        # ReAct 循环 + 上下文管理
│       │   └── tools/        # 内置工具（13 个）
│       │       ├── ppt_plan.rs        # PPT 大纲规划
│       │       ├── ppt_generate.rs    # PPT 生成
│       │       ├── doc_generate.rs    # 文档生成
│       │       ├── sheet_generate.rs  # 表格生成
│       │       ├── chart_generate.rs  # 图表生成
│       │       ├── drawio_generate.rs # 流程图生成
│       │       ├── md_generate.rs     # Markdown 生成
│       │       ├── image_prompt.rs    # 图片提示词 + 生成
│       │       ├── video_generate.rs  # 视频生成（远程）
│       │       ├── local_video.rs     # 视频本地兜底
│       │       ├── agnes_media.rs     # Agnes 图像/视频服务
│       │       ├── web_search.rs      # 联网搜索
│       │       └── mod.rs             # 工具注册表
│       ├── llm/              # OpenAI 兼容 LLM Client（流式 + 非流式）
│       ├── auth/             # JWT 认证中间件
│       ├── db/               # SQLite 连接池 + migration + repository
│       ├── models/           # 数据模型
│       ├── render/           # DOCX / XLSX 纯 Rust 渲染
│       └── routes/           # API 路由（12 个模块）
│           ├── chat.rs       # SSE 流式 Agent 对话
│           ├── auth.rs       # 登录 / 注册 / 验证码
│           ├── session.rs    # 会话管理
│           ├── project.rs    # 项目管理
│           ├── file.rs       # 文件上传 / 下载
│           ├── settings.rs   # 模型切换 / 系统配置
│           ├── dashboard.rs  # 统计面板
│           ├── doc_export.rs # 文档导出
│           ├── notification.rs # 通知
│           ├── embed.rs      # 嵌入式静态资源
│           └── health.rs     # 健康检查
├── frontend/                 # React 前端（~9000 行）
│   └── src/
│       ├── pages/            # Studio / Login / Files
│       ├── components/       # 对话 / 产物展示 / PPT 预览 / 工具栏 / 设置
│       ├── api/              # 统一 API 封装（SSE 解析）
│       ├── stores/           # Zustand 状态管理
│       ├── lib/              # 工具函数
│       ├── types/            # TypeScript 类型定义
│       └── config/           # 前端配置
├── migrations/               # SQL 迁移脚本
├── Dockerfile                # 三阶段构建（前端 → Rust → 运行时）
├── docker-compose-walioffice.yml  # Docker Compose 部署配置
├── build.sh                  # 构建并推送镜像
├── .env.example              # 环境变量模板
└── ARCHITECTURE.md           # 架构设计文档
```

## 🚀 快速开始

### 前置要求

- **Rust** 1.88+（含 cargo）
- **Node.js** 20+（含 npm）
- **ffmpeg**（可选，仅视频本地兜底需要）

### 1. 配置环境变量

```bash
cp .env.example .env
```

编辑 `.env`，填写以下必要配置：

```bash
# ── 安全配置 ──
AIPPT_JWT_SECRET=your-random-secret-at-least-32-chars

# ── 文本 LLM（必填）──
LLM_TEXT_BASE_URL=http://127.0.0.1:8777/v1
LLM_TEXT_API_KEY=your-api-key
LLM_TEXT_MODELS=glm_for_coding

# ── 图片模型（必填）──
LLM_IMAGE_BASE_URL=https://apihub.agnes-ai.com
LLM_IMAGE_API_KEY=your-api-key
LLM_IMAGE_MODELS=agnes-image-2.1-flash

# ── 视频模型（必填）──
LLM_VIDEO_BASE_URL=https://apihub.agnes-ai.com
LLM_VIDEO_API_KEY=your-api-key
LLM_VIDEO_MODELS=agnes-video-v2.0
```

> 完整配置项见 `.env.example`，包括搜索源、数据目录、CORS 等。

### 2. 本地开发

```bash
# 方式一：同时启动前后端（推荐）
npm install
npm run dev
# 前端: http://localhost:5173（热更新）
# 后端: http://localhost:8000

# 方式二：分别启动
cd server && cargo run          # 终端 1：Rust 服务端
cd frontend && npm install && npm run dev  # 终端 2：前端 dev server
```

### 3. 生产构建（单一二进制）

```bash
# 构建前端 → 构建后端（前端产物自动嵌入二进制）
npm run build

# 运行
./server/target/release/walioffice
# 访问 http://localhost:8000
```

### 4. Docker 部署

```bash
# 使用预构建镜像
docker compose -f docker-compose-walioffice.yml up -d

# 或从源码构建
docker build -t walioffice .
docker run -p 8000:8000 -v ./data:/app/data walioffice
```

> 云服务器完整部署流程（含 Nginx + HTTPS）见 [DEPLOY.md](DEPLOY.md)。

## 🔧 Agent 工具

Agent 通过 ReAct 循环自动编排以下工具，用户无需手动选择：

| 工具 | 功能 | 输出格式 |
|------|------|----------|
| `ppt_plan` | 规划 PPT 大纲和结构 | JSON |
| `ppt_generate` | 生成完整 PPT（含视觉设计） | 幻灯片预览 |
| `doc_generate` | 生成文档（报告 / PRD / 计划等） | 可导出 `.docx` |
| `md_generate` | 生成 Markdown 文档 | Markdown |
| `sheet_generate` | 生成结构化表格 | 可导出 `.xlsx` |
| `chart_generate` | 基于数据生成可视化图表 | 图表 |
| `drawio_generate` | 生成流程图 / 架构图 | draw.io XML |
| `image_prompt` | 生成图片提示词并调用文生图 | 图片 |
| `video_generate` | 调用文生视频模型生成视频 | MP4 |
| `web_search` | 联网搜索实时信息 | 搜索结果摘要 |

## 📡 API 接口

| 路由 | 方法 | 说明 |
|------|------|------|
| `/api/auth/login` | POST | 登录（账号密码 / 验证码） |
| `/api/auth/register` | POST | 注册 |
| `/api/auth/me` | GET | 获取当前用户信息 |
| `/api/chat/stream` | POST | SSE 流式 Agent 对话 |
| `/api/chat/sessions` | GET | 会话列表 |
| `/api/session/:id` | GET | 获取会话详情 |
| `/api/session/:id` | DELETE | 删除会话 |
| `/api/project/list` | GET | 项目列表 |
| `/api/project/:id` | GET | 获取项目详情 |
| `/api/file/upload` | POST | 文件上传 |
| `/api/file/download/:id` | GET | 文件下载 |
| `/api/doc/export` | POST | 导出 DOCX |
| `/api/excel/export` | POST | 导出 XLSX |
| `/api/settings/models` | GET | 获取可用模型列表 |
| `/api/settings/model` | PUT | 切换当前模型 |
| `/api/dashboard/stats` | GET | 统计数据 |
| `/api/notification/list` | GET | 通知列表 |
| `/api/health` | GET | 健康检查 |

## 🐳 Docker 构建说明

Dockerfile 采用三阶段构建：

1. **Stage 1 — 前端构建**：`node:20-slim`，npm ci + build
2. **Stage 2 — Rust 构建**：`rust:1.88-bookworm`，cargo build --release（使用 USTC crates 镜像加速）
3. **Stage 3 — 运行时**：`ubuntu:22.04`，仅包含二进制 + ffmpeg + 最小依赖

最终镜像约 50MB，启动即用。

## ⚙️ 环境变量参考

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `AIPPT_JWT_SECRET` | ✅ | — | JWT 签名密钥，生产环境必须修改 |
| `AIPPT_PORT` | ❌ | `8000` | 服务端口 |
| `AIPPT_HOST` | ❌ | `0.0.0.0` | 监听地址 |
| `LLM_TEXT_BASE_URL` | ✅ | — | 文本 LLM API 地址 |
| `LLM_TEXT_API_KEY` | ✅ | — | 文本 LLM API Key |
| `LLM_TEXT_MODELS` | ✅ | — | 可用文本模型列表（逗号分隔） |
| `LLM_IMAGE_BASE_URL` | ✅ | — | 图片模型 API 地址 |
| `LLM_IMAGE_API_KEY` | ✅ | — | 图片模型 API Key |
| `LLM_IMAGE_MODELS` | ✅ | — | 可用图片模型列表 |
| `LLM_VIDEO_BASE_URL` | ✅ | — | 视频模型 API 地址 |
| `LLM_VIDEO_API_KEY` | ✅ | — | 视频模型 API Key |
| `LLM_VIDEO_MODELS` | ✅ | — | 可用视频模型列表 |
| `AIPPT_WEB_SEARCH_PROVIDER` | ❌ | `auto` | 搜索源：`auto` / `searxng` / `duckduckgo` |
| `AIPPT_CORS_ORIGINS` | ❌ | — | CORS 允许来源（逗号分隔） |
| `AIPPT_DATA_DIR` | ❌ | `data` | 数据存储目录 |

> 完整配置见 [`.env.example`](.env.example)。

## 📌 登录说明

- 支持**账号密码注册 / 登录**和**公众号验证码登录**两种方式
- 验证码登录需配置 `WALIOFFICE_X_API_AUTH_LOGIN_URL` 指向验证服务
- 管理后台已暂时移除，启动时不再创建默认管理员账号

## 📄 相关文档

- [架构设计](ARCHITECTURE.md) — 技术分层、模块职责、数据流
- [部署指南](DEPLOY.md) — 云服务器完整部署流程（Docker + Nginx + HTTPS）

## 📜 License

MIT
