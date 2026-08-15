# WaLiOffice

Web 端 AI Agent 智能办公平台 — 单一二进制部署，内置 PPT/文档/表格/流程图生成能力。

## 技术栈

- **服务端**：Rust（axum + tokio + rusqlite）
- **前端**：TypeScript + React 18 + Vite + TailwindCSS
- **LLM**：OpenAI 兼容端点（支持流式）
- **数据库**：嵌入式 SQLite
- **部署**：单一二进制（前端静态资源内嵌进 Rust 二进制）

## 架构

```
WaLiOffice/
├── server/              # Rust 服务端
│   └── src/
│       ├── agent/       # ReAct 循环 + 工具注册表 + 上下文管理
│       │   └── tools/   # 6 个内置工具（PPT/DOC/Sheet/DrawIO/Image）
│       ├── llm/         # OpenAI 兼容 LLM client（非流式 + 流式）
│       ├── db/          # SQLite + repository 模式
│       ├── auth/        # JWT 认证中间件
│       ├── render/      # DOCX/XLSX 渲染（纯 Rust）
│       └── routes/      # axum 路由 + SSE 流式
├── frontend/            # React 前端（构建后嵌入二进制）
│   └── src/
│       ├── pages/       # Studio / Dashboard / Login
│       ├── components/  # ChatPanel / ArtifactPanel / SlidePreview
│       └── stores/      # zustand 状态管理
├── migrations/          # SQL 迁移
├── Dockerfile           # 多阶段构建
└── docker-compose.yml
```

## 快速开始

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env，填写 AIPPT_JWT_SECRET 和 AIPPT_LLM_API_KEY
```

### 2. 本地开发

```bash
# 终端 1：启动 Rust 服务端
cd server && cargo run

# 终端 2：启动前端 dev server（热更新）
cd frontend && npm install && npm run dev
# 访问 http://localhost:5173
```

### 3. 生产构建（单一二进制）

```bash
# 构建前端
cd frontend && npm run build

# 构建后端（会嵌入 frontend/dist/）
cd ../server && cargo build --release

# 运行
./target/release/walioffice
# 访问 http://localhost:8000
```

### 4. Docker 部署

```bash
docker compose up -d
# 访问 http://localhost:8000
```

## API

| 路由 | 方法 | 说明 |
|------|------|------|
| `/api/auth/login` | POST | 登录 |
| `/api/auth/register` | POST | 注册 |
| `/api/auth/me` | GET | 当前用户 |
| `/api/chat/stream` | POST | SSE 流式 Agent 对话 |
| `/api/chat/sessions` | GET | 会话列表 |
| `/api/ppt/projects` | GET | PPT 项目列表 |
| `/api/ppt/project/:id` | GET | 获取 PPT 项目 |
| `/api/doc/export` | POST | 导出 DOCX |
| `/api/excel/export` | POST | 导出 XLSX |
| `/api/dashboard/stats` | GET | 统计 |
| `/api/health` | GET | 健康检查 |

## Agent 工具

| 工具 | 说明 |
|------|------|
| `ppt_plan` | 规划 PPT 大纲 |
| `ppt_generate` | 生成完整 PPT（含视觉设计） |
| `doc_generate` | 生成文档（报告/PRD/计划等） |
| `sheet_generate` | 生成表格数据 |
| `drawio_generate` | 生成 draw.io 图表 |
| `image_prompt` | 生成图片提示词 |

## 默认账号

- 用户名：`admin`
- 密码：`admin123`（可通过 `AIPPT_ADMIN_PASSWORD` 环境变量修改）
