use std::path::PathBuf;

#[tokio::test]
async fn test_config_load() {
    let config_path = PathBuf::from("config.toml");
    let config = galaxy_proxy::config::AppConfig::load(&config_path).unwrap();

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.database.path, "data/galaxy.db");
}

#[tokio::test]
async fn test_database_init() {
    let db_path = "/tmp/galaxy_test/test.db";
    let db_url = format!("sqlite:{}?mode=rwc", db_path);

    // 清理测试数据
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test");

    // 创建测试目录
    std::fs::create_dir_all("/tmp/galaxy_test").unwrap();

    let db = galaxy_proxy::db::Database::new(&db_url).await.unwrap();

    // 验证表已创建
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    .fetch_all(db.pool())
    .await
    .unwrap();

    assert!(tables.contains(&"users".to_string()));
    assert!(tables.contains(&"channels".to_string()));
    assert!(tables.contains(&"groups".to_string()));
    assert!(tables.contains(&"api_keys".to_string()));
    assert!(tables.contains(&"usage_logs".to_string()));
    assert!(tables.contains(&"usage_daily".to_string()));
    assert!(tables.contains(&"settings".to_string()));

    // 验证 settings 表有默认配置
    let settings_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(settings_count > 0, "settings table should have default values");

    // 验证运行时配置加载
    let runtime_config = db.load_runtime_config().await.unwrap();
    assert_eq!(runtime_config.scheduler.top_k, 7);
    assert!(runtime_config.sticky_session.enabled);

    // 清理测试数据
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test");
}

#[test]
fn test_password_hash() {
    let password = "test_password_123";
    let hash = galaxy_proxy::auth::PasswordService::hash_password(password).unwrap();

    assert!(galaxy_proxy::auth::PasswordService::verify_password(password, &hash).unwrap());
    assert!(!galaxy_proxy::auth::PasswordService::verify_password("wrong", &hash).unwrap());
}

#[test]
fn test_jwt_token() {
    let jwt_service = galaxy_proxy::auth::JwtService::new("test_secret", 24);
    let token = jwt_service.generate_token("1", "admin").unwrap();
    let claims = jwt_service.verify_token(&token).unwrap();

    assert_eq!(claims.sub, "1");
    assert_eq!(claims.username, "admin");
}

#[tokio::test]
async fn test_channel_crud() {
    let db_path = "/tmp/galaxy_test_channel/test.db";
    let db_url = format!("sqlite:{}?mode=rwc", db_path);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_channel");
    std::fs::create_dir_all("/tmp/galaxy_test_channel").unwrap();

    let db = galaxy_proxy::db::Database::new(&db_url).await.unwrap();
    let pool = db.pool().clone();

    // 生成 UUID
    let channel_id = uuid::Uuid::now_v7().to_string();

    // 创建渠道
    sqlx::query(
        "INSERT INTO channels (id, name, type, base_url, api_keys) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&channel_id)
    .bind("test-channel")
    .bind("openai_chat")
    .bind("https://api.openai.com")
    .bind(r#"["sk-test"]"#)
    .execute(&pool)
    .await
    .unwrap();

    // 查询渠道
    let fetched_name: String = sqlx::query_scalar(
        "SELECT name FROM channels WHERE id = ?"
    )
    .bind(&channel_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(fetched_name, "test-channel");

    // 删除渠道
    let result = sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(&channel_id)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(result.rows_affected(), 1);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_channel");
}

#[tokio::test]
async fn test_group_crud() {
    let db_path = "/tmp/galaxy_test_group/test.db";
    let db_url = format!("sqlite:{}?mode=rwc", db_path);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_group");
    std::fs::create_dir_all("/tmp/galaxy_test_group").unwrap();

    let db = galaxy_proxy::db::Database::new(&db_url).await.unwrap();
    let pool = db.pool().clone();

    // 先创建一个渠道
    let channel_id = uuid::Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO channels (id, name, type, base_url, api_keys) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&channel_id)
    .bind("test-channel")
    .bind("openai_chat")
    .bind("https://api.openai.com")
    .bind(r#"["sk-test"]"#)
    .execute(&pool)
    .await
    .unwrap();

    // 创建分组
    let group_id = uuid::Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO groups (id, name, mode) VALUES (?, ?, ?)"
    )
    .bind(&group_id)
    .bind("test-group")
    .bind("weighted")
    .execute(&pool)
    .await
    .unwrap();

    // 添加分组项
    let item_id = uuid::Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO group_items (id, group_id, channel_id, model_name, priority, weight) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&item_id)
    .bind(&group_id)
    .bind(&channel_id)
    .bind("gpt-4o")
    .bind(1)
    .bind(100)
    .execute(&pool)
    .await
    .unwrap();

    // 查询分组
    let fetched_name: String = sqlx::query_scalar(
        "SELECT name FROM groups WHERE id = ?"
    )
    .bind(&group_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(fetched_name, "test-group");

    // 查询分组项
    let item_count: i32 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_items WHERE group_id = ?"
    )
    .bind(&group_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(item_count, 1);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_group");
}

#[tokio::test]
async fn test_api_key_crud() {
    let db_path = "/tmp/galaxy_test_api_key/test.db";
    let db_url = format!("sqlite:{}?mode=rwc", db_path);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_api_key");
    std::fs::create_dir_all("/tmp/galaxy_test_api_key").unwrap();

    let db = galaxy_proxy::db::Database::new(&db_url).await.unwrap();
    let pool = db.pool().clone();

    // 创建 API Key
    let key_id = uuid::Uuid::now_v7().to_string();
    let api_key = format!("gp-{}", uuid::Uuid::now_v7());

    sqlx::query(
        "INSERT INTO api_keys (id, name, api_key, enabled) VALUES (?, ?, ?, ?)"
    )
    .bind(&key_id)
    .bind("test-key")
    .bind(&api_key)
    .bind(true)
    .execute(&pool)
    .await
    .unwrap();

    // 查询 API Key
    let fetched_name: String = sqlx::query_scalar(
        "SELECT name FROM api_keys WHERE id = ?"
    )
    .bind(&key_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(fetched_name, "test-key");

    // 验证 API Key
    let enabled: bool = sqlx::query_scalar(
        "SELECT enabled FROM api_keys WHERE api_key = ?"
    )
    .bind(&api_key)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(enabled);

    // 清理测试数据
    let _ = std::fs::remove_dir_all("/tmp/galaxy_test_api_key");
}

#[test]
fn test_openai_chat_transform() {
    use galaxy_proxy::protocol::inbound::Inbound;
    use galaxy_proxy::protocol::openai_chat::OpenAiChatInbound;

    let inbound = OpenAiChatInbound;
    let headers = axum::http::HeaderMap::new();

    let body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "stream": false
    }"#;

    let request = tokio_test::block_on(inbound.transform_request(body.as_bytes(), &headers)).unwrap();

    assert_eq!(request.model, "gpt-4o");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].role, galaxy_proxy::protocol::model::Role::User);
}

#[test]
fn test_anthropic_transform() {
    use galaxy_proxy::protocol::inbound::Inbound;
    use galaxy_proxy::protocol::anthropic::AnthropicInbound;

    let inbound = AnthropicInbound;
    let headers = axum::http::HeaderMap::new();

    let body = r#"{
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    }"#;

    let request = tokio_test::block_on(inbound.transform_request(body.as_bytes(), &headers)).unwrap();

    assert_eq!(request.model, "claude-sonnet-4-20250514");
    assert_eq!(request.messages.len(), 1);
}
