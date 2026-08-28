# WaLiOffice 多租户改造说明

本文档说明对 WaLiOffice 进行的多租户隔离与权限体系升级。

## 一、改造背景

原代码是「用户级（owner_id）数据隔离」的单租户架构，存在以下问题：

1. **无租户概念**：`users.role` 字段存在但从未参与任何鉴权分支，没有 tenant/org/workspace 实体。
2. **越权漏洞**：`chat_stream`（聊天流）通过 `session_id` 获取会话后，未校验 `owner_id`，任意登录用户可向他人会话追加消息、读取上下文。
3. **裸奔接口**：`doc_export` 三个导出 handler 未加鉴权提取器。
4. **无 RBAC**：没有角色权限体系，`system_settings` 表建了但零引用。

## 二、改造内容

### 1. 数据层（多租户表结构）

- 新增 `tenants` 表：`id` / `name` / `slug`（唯一）/ `plan` / `status`。
- `users` 表新增 `tenant_id`（可空，NULL 表示平台级用户/超管），`role` 默认值由 `user` 改为 `member`。
- 业务表 `projects` / `sessions` / `tasks` / `folders` / `files` / `notifications` / `user_settings` 均新增 `tenant_id` 列 + 索引。
- SQLite 与 MySQL 两套 migration 均已同步，并新增**增量迁移**逻辑（对旧库自动 `ALTER TABLE ADD COLUMN`，MySQL 自动建 `tenants` 表）。

### 2. 角色体系（RBAC）

| 角色 | 权限 |
|------|------|
| `super_admin` | 平台级，可管理所有租户、全局用户角色（无 `tenant_id` 归属） |
| `tenant_admin` | 租户级管理员，可管理本租户成员 |
| `member` | 租户普通成员 |

- `User` 新增 `tenant_id` 字段 + `is_super_admin()` / `is_tenant_admin()` 辅助方法。
- JWT `Claims` 新增 `tenant_id`。
- 新增 `ensure_role` 角色校验函数 + `RequireRole` 提取器。

### 3. 越权修复

- **`chat_stream`**：补上 `session.owner_id != user.id` 校验，跨用户会话访问返回 403。
- **`doc_export`**：三个 handler 全部加 `AuthUser` 鉴权；`download_file` 增加路径穿越防护（`file_name()` 提取 + 目录剥离校验）。

### 4. 租户管理接口

新增 `routes/tenant.rs`：

| 路由 | 权限 |
|------|------|
| `GET/POST /api/tenants` | super_admin |
| `GET/PATCH/DELETE /api/tenants/:id` | super_admin |
| `GET /api/tenant/me` | 任意登录用户（返回自己所在租户） |
| `POST /api/tenants/:id/invite-code` | tenant_admin / super_admin（重置邀请码） |
| `GET/POST /api/tenants/:id/members` | tenant_admin / super_admin |
| `POST/DELETE /api/tenants/:id/members/:user_id` | tenant_admin / super_admin |
| `POST /api/users/:user_id/role` | super_admin |

### 5. 注册与邀请码

- 平台**首个注册用户**自动成为 `super_admin`（无租户归属）。
- 每个租户自动生成 `invite_code`（邀请码）。
- **邀请码注册**（`POST /api/auth/register-by-invite`）：成员凭邀请码注册，自动归属对应租户。
- `ALLOW_REGISTER=true` 时，普通注册自动创建个人租户并成为 `tenant_admin`；设为 `false` 则仅允许邀请码注册（收紧准入）。

### 6. 飞书 OAuth 登录

- 端点：`POST /api/auth/feishu/login`（code 换 token + open_id，自动建/找用户并签发 JWT）。
- 前端配置：`GET /api/auth/feishu/config`（返回 app_id / redirect_uri / enabled）。
- 配置项：`FEISHU_APP_ID` / `FEISHU_APP_SECRET` / `FEISHU_REDIRECT_URI`（配齐后前端显示「飞书账号登录」按钮）。
- 飞书用户按 `open_id` 去重（`username=feishu_{open_id}`），首次登录自动建账号。

### 7. 前端管理界面

- 新增 `AdminPage`（`/admin`），仅 `super_admin` / `tenant_admin` 可见入口。
- 超级管理员：租户列表、创建/删除租户、复制/重置邀请码、全局用户角色。
- 租户管理员：查看本租户邀请码、成员管理（添加/移除/角色升降）。
- 登录页升级为多 Tab：验证码登录 / 账号密码登录 / 注册 / 邀请码注册 + 飞书登录按钮。

## 三、启动引导（多租户初始化）

1. 部署后先注册第一个账号 → 自动成为超级管理员。
2. 超管在 `/admin` 界面创建租户，复制邀请码发给成员。
3. 成员用邀请码注册（或飞书登录）自动归属对应租户。
4. 租户管理员在 `/admin` 管理本租户成员与邀请码。

## 四、测试

- `server/tests/multitenancy.rs`：7 个集成测试覆盖租户 CRUD、用户归属、角色升级、租户迁移、RBAC、邀请码流程。
- 全量测试：`cargo test`（19 个用例全部通过）
- 前端：`pnpm build` 构建通过（`webpack/vite` 无错误）。

```bash
cd server
cargo test
```

## 五、MySQL 支持

MySQL 依赖 `sqlx` 的 `mysql` feature（已内置），通过 `DATABASE_URL=mysql://user:password@host:3306/walioffice` 启用。启动时自动执行建表/增量迁移：
- 全新库：完整建表（含 `tenants` + `tenant_id` + `invite_code`）。
- 旧库升级：自动补 `tenants` 表 + `invite_code` 列 + 各业务表 `tenant_id` 列，不丢数据。
