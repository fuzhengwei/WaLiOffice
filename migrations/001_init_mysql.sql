-- ============================================================
-- WaLiOffice MySQL 初始化脚本
-- 每次启动会先 DROP 再 CREATE，清空旧数据重建表结构
-- ============================================================

-- ── 临时关闭外键检查，避免 DROP 顺序冲突 ──
SET FOREIGN_KEY_CHECKS = 0;

DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS projects;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS session_artifacts;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS folders;
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS user_settings;
DROP TABLE IF EXISTS system_settings;

SET FOREIGN_KEY_CHECKS = 1;

-- 用户表
CREATE TABLE users (
    id VARCHAR(36) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    avatar VARCHAR(500),
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 项目表
CREATE TABLE projects (
    id VARCHAR(36) PRIMARY KEY,
    title VARCHAR(500) NOT NULL,
    description VARCHAR(2000),
    tool_kind VARCHAR(100) NOT NULL DEFAULT 'general',
    owner_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_projects_owner (owner_id),
    INDEX idx_projects_kind (tool_kind),
    INDEX idx_projects_updated (updated_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 会话表
CREATE TABLE sessions (
    id VARCHAR(36) PRIMARY KEY,
    owner_id VARCHAR(36) NOT NULL,
    project_id VARCHAR(36),
    tool_kind VARCHAR(100),
    title VARCHAR(500) NOT NULL,
    summary VARCHAR(2000),
    message_count INT NOT NULL DEFAULT 0,
    order_col BIGINT NOT NULL DEFAULT 0,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_sessions_owner (owner_id),
    INDEX idx_sessions_updated (updated_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 会话消息表
CREATE TABLE messages (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL,
    role VARCHAR(50) NOT NULL,
    content LONGTEXT NOT NULL,
    tool_name VARCHAR(255),
    tool_input LONGTEXT,
    tool_output LONGTEXT,
    created_at VARCHAR(50) NOT NULL,
    INDEX idx_messages_session (session_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 会话产物表
CREATE TABLE session_artifacts (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL UNIQUE,
    payload LONGTEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_session_artifacts_session (session_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 任务表
CREATE TABLE tasks (
    id VARCHAR(36) PRIMARY KEY,
    owner_id VARCHAR(36) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description VARCHAR(2000),
    status VARCHAR(50) NOT NULL DEFAULT 'todo',
    priority VARCHAR(50) NOT NULL DEFAULT 'medium',
    due_date VARCHAR(50),
    project_id VARCHAR(36),
    tags VARCHAR(500),
    order_col BIGINT NOT NULL DEFAULT 0,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_tasks_owner (owner_id),
    INDEX idx_tasks_status (status),
    INDEX idx_tasks_priority (priority),
    INDEX idx_tasks_due (due_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 文件夹表
CREATE TABLE folders (
    id VARCHAR(36) PRIMARY KEY,
    owner_id VARCHAR(36) NOT NULL,
    name VARCHAR(500) NOT NULL,
    parent_id VARCHAR(36),
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_folders_owner (owner_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 文件表
CREATE TABLE files (
    id VARCHAR(36) PRIMARY KEY,
    owner_id VARCHAR(36) NOT NULL,
    name VARCHAR(500) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,
    folder_id VARCHAR(36),
    description VARCHAR(2000),
    metadata VARCHAR(4000),
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_files_owner (owner_id),
    INDEX idx_files_folder (folder_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 通知表
CREATE TABLE notifications (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    type VARCHAR(100) NOT NULL,
    title VARCHAR(500) NOT NULL,
    content VARCHAR(2000),
    is_read TINYINT NOT NULL DEFAULT 0,
    link VARCHAR(500),
    created_at VARCHAR(50) NOT NULL,
    INDEX idx_notifications_user (user_id),
    INDEX idx_notifications_read (user_id, is_read)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 用户设置表
CREATE TABLE user_settings (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL UNIQUE,
    payload LONGTEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL,
    INDEX idx_user_settings_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 系统设置表
CREATE TABLE system_settings (
    `key` VARCHAR(255) PRIMARY KEY,
    payload LONGTEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    updated_at VARCHAR(50) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
