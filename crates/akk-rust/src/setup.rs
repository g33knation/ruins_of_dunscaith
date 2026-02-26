use anyhow::{Result, Context};

pub async fn execute() -> Result<()> {
    println!("🛠️  Initializing Ruins of Dunscaith environment...");
    
    // 1. Check for .env
    if !std::path::Path::new(".env").exists() {
        println!("Creating .env from .env.example...");
        if std::path::Path::new(".env.example").exists() {
            std::fs::copy(".env.example", ".env")?;
        } else {
            println!("⚠️ .env.example not found! Please create a .env file manually.");
        }
    }

    // 2. Database checks (Natively absorbing setup_db.sh)
    println!("Verifying database setup...");
    let mut cmd = tokio::process::Command::new("sudo");
    cmd.args(&["-u", "postgres", "psql", "-c", "ALTER DATABASE postgres REFRESH COLLATION VERSION;"]);
    let _ = cmd.status().await; // Non-critical

    // Create eqemu user
    let mut cmd = tokio::process::Command::new("sudo");
    cmd.args(&["-u", "postgres", "psql", "-c", "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'eqemu') THEN CREATE ROLE eqemu LOGIN PASSWORD 'eqemupass'; END IF; END $$;"]);
    let _ = cmd.status().await;

    // Create peq database
    let mut cmd = tokio::process::Command::new("sudo");
    cmd.args(&["-u", "postgres", "createdb", "-O", "eqemu", "peq"]);
    let _ = cmd.status().await; // Might fail if exists, that's fine

    // Apply migrations
    println!("Applying SQL migrations...");
    let migrations = vec![
        "migrations/20240121000001_create_tables.sql",
        "migrations/20240121000002_world_redirection.sql",
        "migrations/20240121000003_full_peq_schema.sql",
    ];

    for migration in migrations {
        if std::path::Path::new(migration).exists() {
            println!("Applying {}...", migration);
            let mut cmd = tokio::process::Command::new("psql");
            cmd.env("PGPASSWORD", "eqemupass")
               .args(&["-h", "127.0.0.1", "-U", "eqemu", "-d", "peq", "-f", migration]);
            let status = cmd.status().await.context("Failed to run migration")?;
            if !status.success() {
                println!("⚠️ Warning: Migration {} failed. Check if tables already exist.", migration);
            }
        }
    }

    println!("✅ Setup complete. Use 'akk-rust up' to start the services.");
    Ok(())
}
