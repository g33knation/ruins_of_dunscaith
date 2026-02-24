use sqlx::mysql::MySqlPool;
use std::env;
use dotenv::dotenv;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = MySqlPool::connect(&database_url).await?;

    println!("🌱 Seeding 'admin' account...");

    let res = sqlx::query!(
        "INSERT INTO account (name, status, lsaccount_id, password) VALUES (?, ?, ?, ?)",
        "admin", 255, 1, "password"
    )
    .execute(&pool)
    .await;

    match res {
        Ok(_) => println!("✅ Account 'admin' created successfully!"),
        Err(e) => println!("❌ Failed to create account: {}", e),
    }
    
    Ok(())
}
