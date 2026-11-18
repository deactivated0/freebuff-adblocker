pub mod backup;
pub mod patch;
pub mod process;
pub mod settings;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_default_binary_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/manicode/freebuff")
    } else {
        PathBuf::from(".config/manicode/freebuff")
    }
}

pub fn check_elf_format(path: &Path) -> io::Result<String> {
    let data = fs::read(path)?;
    if data.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "File is too small to be an ELF binary",
        ));
    }

    if &data[0..4] != b"\x7fELF" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "File missing ELF magic bytes",
        ));
    }

    let class = match data[4] {
        1 => "32-bit",
        2 => "64-bit",
        _ => "Unknown class",
    };

    let endian = match data[5] {
        1 => "LSB (little endian)",
        2 => "MSB (big endian)",
        _ => "Unknown endianness",
    };

    Ok(format!("ELF {} {}, size {} bytes", class, endian, data.len()))
}

pub fn get_installed_version(binary_path: &Path) -> Option<String> {
    // 1. Try reading freebuff-metadata.json next to binary
    if let Some(parent) = binary_path.parent() {
        let meta_path = parent.join("freebuff-metadata.json");
        if let Ok(content) = fs::read_to_string(&meta_path) {
            let mut parser = settings::JsonParser::new(&content);
            if let Ok(settings::JsonValue::Object(entries)) = parser.parse() {
                for (k, v) in entries {
                    if k == "version" {
                        if let settings::JsonValue::String(ver) = v {
                            return Some(ver);
                        }
                    }
                }
            }
        }
    }

    // 2. Try running binary --version
    if let Ok(output) = Command::new(binary_path).arg("--version").output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ver.is_empty() {
                return Some(ver);
            }
        }
    }

    None
}
