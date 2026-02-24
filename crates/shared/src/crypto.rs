use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use anyhow::Result;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Hashing failed: {}", e))?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Invalid hash format: {}", e))?;
    
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(anyhow::anyhow!("Verification failed: {}", e)),
    }
}
