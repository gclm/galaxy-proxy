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
