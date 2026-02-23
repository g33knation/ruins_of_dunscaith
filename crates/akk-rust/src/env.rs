use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::Result;
use rand::distr::Alphanumeric;
use rand::RngExt;

pub fn scramble(target_key: Option<&str>) -> Result<()> {
    let fields_to_scramble = [
        "MARIADB_ROOT_PASSWORD",
        "MARIADB_PASSWORD",
        "PHPMYADMIN_PASSWORD",
        "PEQ_EDITOR_PROXY_PASSWORD",
        "SERVER_PASSWORD",
        "FTP_QUESTS_PASSWORD",
        "SPIRE_ADMIN_PASSWORD",
        "PEQ_EDITOR_PASSWORD",
    ];

    let dot_env_path = Path::new(".env");
    if !dot_env_path.exists() {
        println!(".env file not found. Run 'transplant' first.");
        return Ok(());
    }

    let file = File::open(dot_env_path)?;
    let reader = BufReader::new(file);
    let mut new_content = String::new();

    for line in reader.lines() {
        let line = line?;
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            let should_scramble = fields_to_scramble.contains(&key) && 
                (target_key.is_none() || target_key == Some(key));

            if should_scramble && value == "<template>" {
                let new_secret: String = rand::rng()
                    .sample_iter(&Alphanumeric)
                    .take(31)
                    .map(char::from)
                    .collect();
                println!("Scrambling [{}]", key);
                new_content.push_str(&format!("{}={}\n", key, new_secret));
            } else {
                new_content.push_str(&line);
                new_content.push('\n');
            }
        } else {
            new_content.push_str(&line);
            new_content.push('\n');
        }
    }

    fs::write(dot_env_path, new_content)?;
    println!("Wrote updated config to [.env]");
    Ok(())
}

pub fn transplant() -> Result<()> {
    let example_path = Path::new(".env.example");
    let dot_env_path = Path::new(".env");

    if !example_path.exists() {
        anyhow::bail!(".env.example not found");
    }

    let mut current_values = HashMap::new();
    if dot_env_path.exists() {
        let file = File::open(dot_env_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                current_values.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    let example_file = File::open(example_path)?;
    let example_reader = BufReader::new(example_file);
    let mut new_content = String::new();

    for line in example_reader.lines() {
        let line = line?;
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if let Some(current_val) = current_values.get(key) {
                if current_val != value {
                    new_content.push_str(&format!("{}={}\n", key, current_val));
                    continue;
                }
            }
        }
        new_content.push_str(&line);
        new_content.push('\n');
    }

    fs::write(dot_env_path, new_content)?;
    println!("Wrote updated config to [.env]");
    Ok(())
}
