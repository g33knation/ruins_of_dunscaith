mod net;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();
    
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&database_url).await?;
    let pool = std::sync::Arc::new(pool);
    
    // Start Login Server Listener
    net::start_server(pool).await?;
    
    Ok(())
}
