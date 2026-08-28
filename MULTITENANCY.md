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
| `GET/POST /api/tenants/:id/members` | tenant_admin / super_admin |
| `POST/DELETE /api/tenants/:id/members/:user_id` | tenant_admin / super_admin |
| `POST /api/users/:user_id/role` | super_admin |

### 5. 注册逻辑

- 平台**首个注册用户**自动成为 `super_admin`（无租户归属）。
- 后续用户注册自动创建**个人租户**并成为 `tenant_admin`。
- 支持在注册时指定 `tenant_id` 归属已有租户（成员身份）。

## 三、启动引导（多租户初始化）

1. 部署后先注册第一个账号 → 自动成为超级管理员。
2. 超管调用 `POST /api/tenants` 创建租户。
3. 租户管理员邀请成员（`POST /api/tenants/:id/members`）或成员注册时带 `tenant_id` 加入。

## 四、测试

- `server/tests/multitenancy.rs`：6 个集成测试覆盖租户 CRUD、用户归属、角色升级、租户迁移、RBAC 辅助函数。
- 全量测试：`cargo test`（18 个用例全部通过）。

```bash
cd server
cargo test
```

## 五、Microsoft MySQL 支持

MySQL 依赖 `sqlx` 的 `mysql` feature（已内置），通过 `DATABASE_URL=mysql://user:password@host:3306/walioffice` 启用。启动时自动执行建表/增量迁移：
- 全新库：完整建表（含 `tenants` + `tenant_id`）。
- 旧库升级：自动补 `tenants` 表 + 各业务表 `tenant_id` 列，不丢数据。
