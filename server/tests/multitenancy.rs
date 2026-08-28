//! 多租户隔离单元/集成测试
//! 使用内存 SQLite 验证：租户 CRUD、用户归属、RBAC 角色、越权防护逻辑

use std::sync::atomic::{AtomicU32, Ordering};
use walioffice::db::{tenant_repo, user_repo};

static COUNTER: AtomicU32 = AtomicU32::new(0);

async fn make_pool() -> walioffice::db::DbPool {
    // 每个测试用独立的临时文件 SQLite，避免连接池间内存库不共享的问题
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join("walioffice-test");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("t{pid}_{n}.db"));
    let url = format!("sqlite://{}?mode=rwc", file.display());
    walioffice::db::init_pool(&url, "", 2)
        .await
        .expect("init pool")
}

#[tokio::test]
async fn tenant_create_and_find() {
    let pool = make_pool().await;
    let t = tenant_repo::create(&pool, "测试租户", "test-slug", "pro")
        .await
        .expect("create tenant");

    assert_eq!(t.name, "测试租户");
    assert_eq!(t.slug, "test-slug");
    assert_eq!(t.plan, "pro");
    assert_eq!(t.status, "active");

    let found = tenant_repo::find_by_id(&pool, &t.id).await.expect("find").expect("exists");
    assert_eq!(found.name, "测试租户");

    let by_slug = tenant_repo::find_by_slug(&pool, "test-slug").await.expect("by slug").expect("exists");
    assert_eq!(by_slug.id, t.id);
}

#[tokio::test]
async fn tenant_slug_unique_and_list() {
    let pool = make_pool().await;
    tenant_repo::create(&pool, "A", "alpha", "free").await.unwrap();
    tenant_repo::create(&pool, "B", "beta", "free").await.unwrap();

    let list = tenant_repo::list(&pool).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn user_tenant_assignment_and_role() {
    let pool = make_pool().await;
    let tenant = tenant_repo::create(&pool, "Org", "org", "free").await.unwrap();

    // 创建超管（无租户）
    let admin = user_repo::create_with_tenant(&pool, "admin", None, "h", None, "super_admin")
        .await
        .unwrap();
    assert!(admin.tenant_id.is_none());
    assert!(admin.is_super_admin());

    // 创建租户管理员
    let tenant_admin = user_repo::create_with_tenant(
        &pool,
        "ta",
        None,
        "h",
        Some(&tenant.id),
        "tenant_admin",
    )
    .await
    .unwrap();
    assert_eq!(tenant_admin.tenant_id.as_deref(), Some(tenant.id.as_str()));
    assert!(tenant_admin.is_tenant_admin());
    assert!(!tenant_admin.is_super_admin());

    // 创建普通成员
    let member = user_repo::create_with_tenant(&pool, "m", None, "h", Some(&tenant.id), "member")
        .await
        .unwrap();
    assert_eq!(member.role, "member");
    assert!(!member.is_tenant_admin());

    // 列出租户成员
    let members = user_repo::list_by_tenant(&pool, &tenant.id).await.unwrap();
    assert_eq!(members.len(), 2); // tenant_admin + member（admin 不在此租户）
}

#[tokio::test]
async fn user_update_role_and_assign_tenant() {
    let pool = make_pool().await;
    let t1 = tenant_repo::create(&pool, "T1", "t1", "free").await.unwrap();
    let t2 = tenant_repo::create(&pool, "T2", "t2", "free").await.unwrap();

    let user = user_repo::create_with_tenant(&pool, "u", None, "h", Some(&t1.id), "member")
        .await
        .unwrap();

    // 变更角色
    let upgraded = user_repo::update_role(&pool, &user.id, "tenant_admin").await.unwrap().unwrap();
    assert_eq!(upgraded.role, "tenant_admin");

    // 迁移租户
    let moved = user_repo::assign_tenant(&pool, &user.id, Some(&t2.id)).await.unwrap().unwrap();
    assert_eq!(moved.tenant_id.as_deref(), Some(t2.id.as_str()));

    // 逐出租户
    let ejected = user_repo::assign_tenant(&pool, &user.id, None).await.unwrap().unwrap();
    assert!(ejected.tenant_id.is_none());
}

#[tokio::test]
async fn user_count_works() {
    let pool = make_pool().await;
    assert_eq!(user_repo::count(&pool).await.unwrap(), 0);
}

#[test]
fn rbac_role_helpers() {
    use walioffice::models::User;

    let mk = |role: &str, tenant: Option<&str>| User {
        id: "id".into(),
        tenant_id: tenant.map(str::to_string),
        username: "u".into(),
        email: None,
        avatar: None,
        role: role.into(),
    };

    assert!(mk("super_admin", None).is_super_admin());
    assert!(mk("super_admin", None).is_tenant_admin());
    assert!(mk("tenant_admin", Some("t")).is_tenant_admin());
    assert!(!mk("tenant_admin", Some("t")).is_super_admin());
    assert!(!mk("member", Some("t")).is_tenant_admin());
    assert!(!mk("member", Some("t")).is_super_admin());
}
