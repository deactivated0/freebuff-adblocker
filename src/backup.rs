use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub struct BackupManager {
    binary_path: PathBuf,
    binary_backup_path: PathBuf,
    settings_path: PathBuf,
    settings_backup_path: PathBuf,
}

impl BackupManager {
    pub fn new(binary_path: &Path, settings_path: &Path) -> Self {
        Self {
            binary_path: binary_path.to_path_buf(),
            binary_backup_path: binary_path.with_extension("bak"),
            settings_path: settings_path.to_path_buf(),
            settings_backup_path: settings_path.with_extension("json.bak"),
        }
    }

    pub fn binary_backup_exists(&self) -> bool {
        self.binary_backup_path.exists()
    }

    pub fn settings_backup_exists(&self) -> bool {
        self.settings_backup_path.exists()
    }

    pub fn backup_binary(&self) -> io::Result<bool> {
        if !self.binary_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Target binary does not exist: {}", self.binary_path.display()),
            ));
        }

        // If backup already exists, preserve the original clean backup
        if self.binary_backup_path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(&self.binary_path)?;
        let permissions = metadata.permissions();

        // Write to temp backup first, then rename
        let tmp_backup = self.binary_backup_path.with_extension(format!("tmp.{}", std::process::id()));
        fs::copy(&self.binary_path, &tmp_backup)?;
        fs::set_permissions(&tmp_backup, permissions)?;
        fs::rename(&tmp_backup, &self.binary_backup_path)?;

        Ok(true)
    }

    pub fn backup_settings(&self) -> io::Result<bool> {
        if !self.settings_path.exists() {
            return Ok(false);
        }

        if self.settings_backup_path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(&self.settings_path)?;
        let permissions = metadata.permissions();

        let tmp_backup = self.settings_backup_path.with_extension(format!("tmp.{}", std::process::id()));
        fs::copy(&self.settings_path, &tmp_backup)?;
        fs::set_permissions(&tmp_backup, permissions)?;
        fs::rename(&tmp_backup, &self.settings_backup_path)?;

        Ok(true)
    }

    pub fn write_atomic_binary(&self, data: &[u8]) -> io::Result<()> {
        let parent = self.binary_path.parent().unwrap_or_else(|| Path::new("."));
        let tmp_file = parent.join(format!(
            ".freebuff_patch.tmp.{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        // Get permissions from existing binary if possible, otherwise default to 0755
        let mode = if self.binary_path.exists() {
            fs::metadata(&self.binary_path)?.permissions().mode()
        } else {
            0o755
        };

        // Write content
        fs::write(&tmp_file, data)?;

        // Ensure executable permissions
        fs::set_permissions(&tmp_file, Permissions::from_mode(mode))?;

        // Atomic replace
        fs::rename(&tmp_file, &self.binary_path)?;

        Ok(())
    }

    pub fn restore_binary(&self) -> io::Result<()> {
        if !self.binary_backup_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Binary backup not found at {}", self.binary_backup_path.display()),
            ));
        }

        // Validate backup has ELF header
        let backup_header = fs::read(&self.binary_backup_path)?;
        if backup_header.len() < 4 || &backup_header[0..4] != b"\x7fELF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Backup file does not have a valid ELF header",
            ));
        }

        let permissions = fs::metadata(&self.binary_backup_path)?.permissions();

        let tmp_file = self.binary_path.with_extension(format!("restore.tmp.{}", std::process::id()));
        fs::copy(&self.binary_backup_path, &tmp_file)?;
        fs::set_permissions(&tmp_file, permissions)?;
        fs::rename(&tmp_file, &self.binary_path)?;

        Ok(())
    }

    pub fn restore_settings(&self) -> io::Result<bool> {
        if !self.settings_backup_path.exists() {
            return Ok(false);
        }

        let permissions = fs::metadata(&self.settings_backup_path)?.permissions();
        let tmp_file = self.settings_path.with_extension(format!("restore.tmp.{}", std::process::id()));
        fs::copy(&self.settings_backup_path, &tmp_file)?;
        fs::set_permissions(&tmp_file, permissions)?;
        fs::rename(&tmp_file, &self.settings_path)?;

        Ok(true)
    }

    pub fn binary_backup_path(&self) -> &Path {
        &self.binary_backup_path
    }

    pub fn settings_backup_path(&self) -> &Path {
        &self.settings_backup_path
    }
}
