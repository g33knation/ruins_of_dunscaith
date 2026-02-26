use anyhow::Result;
use std::path::Path;
use std::fs;

pub fn execute() -> Result<()> {
    println!("🧹 Commencing monorepo cleanup...");

    let paths_to_remove = vec![
        "src",
        "akk-stack-legacy",
        "temp_rof2_structs.h",
        "setup_db.sh",
        "start_servers.sh",
    ];

    for path_str in paths_to_remove {
        let path = Path::new(path_str);
        if path.exists() {
            if path.is_dir() {
                println!("Removing legacy directory: {}", path_str);
                fs::remove_dir_all(path)?;
            } else {
                println!("Removing legacy file: {}", path_str);
                fs::remove_file(path)?;
            }
        } else {
            println!("Skipping {}: already removed or does not exist.", path_str);
        }
    }

    println!("✅ Monorepo cleanup complete.");
    Ok(())
}
