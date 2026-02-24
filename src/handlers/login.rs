use sqlx::{MySqlPool, Error, Row};
use crate::packets::login::LoginPacket;

/// Converts C++ HandleLogin logic to Rust.
/// Verifies credentials against the database.
pub async fn handle_login(pool: &MySqlPool, packet: &LoginPacket) -> Result<bool, Error> {
    let username = packet.get_name();
    let password = packet.get_password();

    // Equivalent to: SELECT ... FROM account WHERE name = ...
    // Note: In production, never store or compare plain text passwords. 
    // This mimics legacy EQEmu behavior or assumes internal hashing logic.
    // Convert compile-time query! (which requires a live DB) to runtime query
    let row = sqlx::query(
        r#"
        SELECT id, name, password, status 
        FROM account 
        WHERE name = ?
        "#
    )
    .bind(&username)
    .fetch_optional(pool)
    .await?;

    if let Some(acc_row) = row {
        let acc_name: String = acc_row.get("name");
        let acc_password: String = acc_row.get("password");
        let acc_id: i32 = acc_row.get("id");

        if acc_password == password {
            println!("Login successful for account: {} (ID: {})", acc_name, acc_id);
            return Ok(true);
        }
    }

    println!("Login failed for user: {}", username);
    Ok(false)
}
