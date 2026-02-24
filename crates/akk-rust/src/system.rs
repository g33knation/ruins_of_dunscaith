use std::env;
use anyhow::Result;
use dotenvy::dotenv;

pub fn info() -> Result<()> {
    dotenv().ok();

    println!("----------------------------------");
    println!("> Server Info");
    println!("----------------------------------");

    // Display key env vars
    let ip = env::var("IP_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "Not Set".to_string());
    let tz = env::var("TZ").unwrap_or_else(|_| "UTC".to_string());

    println!("IP Address:  {}", ip);
    println!("Timezone:    {}", tz);
    println!("DB URL:      {}", db_url);
    
    println!("----------------------------------");
    println!("> Component Status");
    println!("----------------------------------");
    // In a real implementation, we would query docker-compose ps
    println!("(Run 'akk-rust up' to start services)");

    Ok(())
}
