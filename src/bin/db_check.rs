use sqlx::mysql::MySqlPool;
use std::env;
use dotenv::dotenv;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("🔌 Connecting to database: {}", database_url);

    let pool = MySqlPool::connect(&database_url).await?;
    println!("✅ Connected successfully.");

    println!("\n🔍 Querying 'account' table...");
    
    struct AccountMsg {
        id: i32,
        name: String,
        status: i32,
        lsaccount_id: Option<u32>,
    }

    let accounts = sqlx::query_as!(
        AccountMsg,
        "SELECT id, name, status, lsaccount_id FROM account"
    )
    .fetch_all(&pool)
    .await?;

    if accounts.is_empty() {
        println!("❌ NO ACCOUNTS FOUND IN 'account' TABLE!");
    } else {
        println!("📝 Found {} account(s):", accounts.len());
        println!("{:<5} | {:<20} | {:<6} | {:<10}", "ID", "NAME", "STATUS", "LS_ID");
        println!("{}", "-".repeat(50));
        for acc in accounts {
            println!("{:<5} | {:<20} | {:<6} | {:<10?}", acc.id, acc.name, acc.status, acc.lsaccount_id);
        }
    }

    Ok(())
}
