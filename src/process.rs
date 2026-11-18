use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
}

pub fn find_running_freebuff_processes() -> Vec<ProcessInfo> {
    let mut results = Vec::new();
    let current_pid = std::process::id();

    // Scan /proc
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                if pid == current_pid {
                    continue;
                }

                let proc_path = entry.path();
                let comm_path = proc_path.join("comm");
                let cmdline_path = proc_path.join("cmdline");

                let comm = fs::read_to_string(&comm_path)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let cmdline = fs::read(&cmdline_path)
                    .map(|bytes| {
                        bytes
                            .split(|&b| b == 0)
                            .map(|s| String::from_utf8_lossy(s))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();

                // Check exact match on comm == "freebuff" or executable path ending in "/freebuff"
                let is_freebuff = comm == "freebuff"
                    || is_cmdline_freebuff(&cmdline);

                // Make sure we never match freebuff-patcher itself
                let is_patcher = comm.contains("freebuff-patcher")
                    || cmdline.contains("freebuff-patcher");

                if is_freebuff && !is_patcher {
                    results.push(ProcessInfo {
                        pid,
                        name: comm,
                        cmdline,
                    });
                }
            }
        }
    }

    // Fallback check with pgrep if /proc was empty or inaccessible
    if results.is_empty() {
        if let Ok(output) = Command::new("pgrep").arg("-x").arg("freebuff").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Ok(pid) = line.trim().parse::<u32>() {
                        if pid != current_pid && !results.iter().any(|p| p.pid == pid) {
                            results.push(ProcessInfo {
                                pid,
                                name: "freebuff".to_string(),
                                cmdline: "freebuff".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    results
}

fn is_cmdline_freebuff(cmdline: &str) -> bool {
    let first_arg = cmdline.split_whitespace().next().unwrap_or("");
    let path = Path::new(first_arg);
    if let Some(file_name) = path.file_name() {
        return file_name == "freebuff";
    }
    false
}

pub fn check_process_safety() -> io::Result<()> {
    let procs = find_running_freebuff_processes();
    if !procs.is_empty() {
        let pids: Vec<String> = procs.iter().map(|p| format!("PID {}", p.pid)).collect();
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Freebuff is currently running ({}). Please close Freebuff before patching.",
                pids.join(", ")
            ),
        ));
    }
    Ok(())
}
