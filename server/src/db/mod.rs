pub mod file_repo;
pub mod notification_repo;
pub mod project_repo;
pub mod session_repo;
pub mod settings_repo;
pub mod user_repo;

use anyhow::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::AnyPool;
use std::fs;
use tracing::info;

pub type DbPool = AnyPool;

pub async fn init_pool(database_url: &str, data_dir: &str, max_connections: u32) -> Result<DbPool> {
    // 安装 sqlx any 驱动（必须在使用 AnyPool 前调用）
    sqlx::any::install_default_drivers();

    if database_url.starts_with("mysql://") {
        // 先用 MySqlPool 跑迁移
        let mysql_pool = MySqlPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        run_migrations_mysql(&mysql_pool).await?;
        info!("📦 MySQL 数据库已初始化: {}", mask_url_password(database_url));

        // 再用 AnyPool 连接
        let any_pool = AnyPool::connect(database_url).await?;
        Ok(any_pool)
    } else {
        // SQLite
        if !data_dir.is_empty() {
            fs::create_dir_all(data_dir)?;
        }
        let sqlite_pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        run_migrations_sqlite(&sqlite_pool).await?;
        info!("📦 SQLite 数据库已初始化: {}", database_url);

        let any_pool = AnyPool::connect(database_url).await?;
        Ok(any_pool)
    }
}

async fn run_migrations_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let sql = include_str!("../../../migrations/001_init.sql");
    sqlx::raw_sql(sql).execute(pool).await?;

    // 检查 sessions 表是否有 order_col 列
    let has_order_col: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('sessions') WHERE name = 'order_col'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_order_col {
        sqlx::query("ALTER TABLE sessions ADD COLUMN order_col INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    Ok(())
}

async fn run_migrations_mysql(pool: &sqlx::MySqlPool) -> Result<()> {
    // 检查 users 表是否已存在，存在则跳过迁移（避免每次重启清空数据）
    let table_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'users'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if table_exists {
        info!("📦 MySQL 表已存在，跳过迁移");
        return Ok(());
    }

    let sql = include_str!("../../../migrations/001_init_mysql.sql");
    sqlx::raw_sql(sql).execute(pool).await?;
    Ok(())
}

/// 隐藏 URL 中的密码部分，用于日志输出
fn mask_url_password(url: &str) -> String {
    if let Some(start) = url.find("://") {
        let scheme_end = start + 3;
        if let Some(at_pos) = url[scheme_end..].find('@') {
            let user_start = scheme_end;
            let user_end = scheme_end + at_pos;
            let password_start = url[user_start..].find(':');
            if let Some(rel_pw_start) = password_start {
                let pw_start = user_start + rel_pw_start;
                return format!("{}{}***@{}", &url[..pw_start], "", &url[user_end + 1..]);
            }
        }
    }
    url.to_string()
}
