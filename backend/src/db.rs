use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn init_pool(database_url: &str) -> MySqlPool {
    MySqlPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to create MySQL pool")
}
