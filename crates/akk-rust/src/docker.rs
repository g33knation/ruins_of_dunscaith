use std::process::Stdio;
use anyhow::{Result, Context};
use tokio::process::Command;

pub async fn up(detached: bool) -> Result<()> {
    let mut cmd = Command::new("docker-compose");
    cmd.arg("up");
    if detached {
        cmd.arg("-d");
    }
    
    cmd.stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let status = cmd.status().await
        .context("Failed to execute docker-compose up")?;

    if !status.success() {
        anyhow::bail!("docker-compose up failed with status {}", status);
    }

    Ok(())
}

pub async fn down() -> Result<()> {
    let mut cmd = Command::new("docker-compose");
    cmd.arg("down");
    
    cmd.stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let status = cmd.status().await
        .context("Failed to execute docker-compose down")?;

    if !status.success() {
        anyhow::bail!("docker-compose down failed with status {}", status);
    }

    Ok(())
}

pub async fn restart() -> Result<()> {
    let mut cmd = Command::new("docker-compose");
    cmd.arg("restart");
    
    cmd.stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let status = cmd.status().await
        .context("Failed to execute docker-compose restart")?;

    if !status.success() {
        anyhow::bail!("docker-compose restart failed with status {}", status);
    }

    Ok(())
}
