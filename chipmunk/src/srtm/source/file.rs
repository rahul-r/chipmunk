use std::fs;
use std::path::PathBuf;

const CACHE_DIR: &str = "/tmp/chipmunk-cache";

pub fn exists(name: &str) -> bool {
    let hgt_file = PathBuf::from(CACHE_DIR).join(format!("{name}.hgt"));
    hgt_file.exists()
}

pub fn load(name: &str) -> anyhow::Result<Vec<u8>> {
    let hgt_file = PathBuf::from(CACHE_DIR).join(format!("{name}.hgt"));

    let data = fs::read(&hgt_file)?;
    let num_bytes_read = data.len();

    log::info!("File loaded from {hgt_file:?} ({num_bytes_read} bytes)");
    Ok(data)
}
