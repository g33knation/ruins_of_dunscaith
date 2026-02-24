use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::env;
use std::error::Error;

pub async fn establish_connection() -> Result<MySqlPool, Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    Ok(pool)
}
