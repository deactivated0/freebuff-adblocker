use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use freebuff_patcher::backup::BackupManager;
use freebuff_patcher::patch::{self, PatchScanResult, SignatureState};
use freebuff_patcher::process;
use freebuff_patcher::settings;
use freebuff_patcher::{check_elf_format, get_default_binary_path, get_installed_version};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Check,
    Apply,
    Verify,
    Restore,
    Help,
}

struct Config {
    action: Action,
    binary_path: PathBuf,
    settings_path: PathBuf,
    force: bool,
}

fn parse_args() -> Result<Config, String> {
    let mut action = None;
    let mut binary_path = None;
    let mut settings_path = None;
    let mut force = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--check" => {
                action = Some(Action::Check);
            }
            "--apply" => {
                action = Some(Action::Apply);
            }
            "--verify" => {
                action = Some(Action::Verify);
            }
            "--restore" => {
                action = Some(Action::Restore);
            }
            "-h" | "--help" | "help" => {
                action = Some(Action::Help);
            }
            "--force" => {
                force = true;
            }
            "--binary" | "--target" => {
                i += 1;
                if i >= args.len() {
                    return Err(format!("Option '{}' requires an argument", arg));
                }
                binary_path = Some(PathBuf::from(&args[i]));
            }
            "--settings" => {
                i += 1;
                if i >= args.len() {
                    return Err(format!("Option '{}' requires an argument", arg));
                }
                settings_path = Some(PathBuf::from(&args[i]));
            }
            unknown if unknown.starts_with('-') => {
                return Err(format!("Unknown option: {}", unknown));
            }
            _ => {
                return Err(format!("Unexpected argument: {}", arg));
            }
        }
        i += 1;
    }

    Ok(Config {
        action: action.unwrap_or(Action::Check), // Default to dry-run check
        binary_path: binary_path.unwrap_or_else(get_default_binary_path),
        settings_path: settings_path.unwrap_or_else(settings::get_default_settings_path),
        force,
    })
}

fn print_help() {
    println!("Freebuff Terminal: Native Rust Binary Patcher");
    println!("Disables all advertisement surfaces and updates settings for Freebuff on Linux x86_64.\n");
    println!("USAGE:");
    println!("    freebuff-patcher [OPTIONS] [COMMAND]\n");
    println!("COMMANDS:");
    println!("    --check      Inspect binary and settings without making any changes (default dry-run)");
    println!("    --apply      Safely patch binary, update settings, and back up originals");
    println!("    --verify     Validate that all 13 patches and settings are in effect");
    println!("    --restore    Roll back the binary and settings from previous backups");
    println!("    --help       Print this help guide\n");
    println!("OPTIONS:");
    println!("    --target <PATH>    Path to freebuff binary (default: ~/.config/manicode/freebuff)");
    println!("    --settings <PATH>  Path to settings.json (default: ~/.config/manicode/settings.json)");
    println!("    --force            Bypass running process safety check\n");
    println!("EXIT CODES:");
    println!("    0    Operation succeeded / check clean");
    println!("    1    Error encountered / signature missing / verification failed");
    println!("    2    Invalid command line arguments");
}

fn main() -> ExitCode {
    let config = match parse_args() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Error: {}", err);
            eprintln!("Run 'freebuff-patcher --help' for usage details.");
            return ExitCode::from(2);
        }
    };

    if config.action == Action::Help {
        print_help();
        return ExitCode::SUCCESS;
    }

    println!("============================================================");
    println!("       Freebuff Terminal Native Rust Ad Patcher v0.1.0      ");
    println!("============================================================");
    println!("Target Binary:   {}", config.binary_path.display());
    println!("Settings File:   {}", config.settings_path.display());

    match config.action {
        Action::Check => cmd_check(&config),
        Action::Apply => cmd_apply(&config),
        Action::Verify => cmd_verify(&config),
        Action::Restore => cmd_restore(&config),
        Action::Help => unreachable!(),
    }
}

fn cmd_check(config: &Config) -> ExitCode {
    println!("\n[1/4] Checking Binary Presence and Format...");
    if !config.binary_path.exists() {
        eprintln!("[-] Error: Target executable not found at {}", config.binary_path.display());
        return ExitCode::from(1);
    }

    match check_elf_format(&config.binary_path) {
        Ok(info) => println!("  [+] Binary format: {}", info),
        Err(e) => {
            eprintln!("  [-] ELF format check failed: {}", e);
            return ExitCode::from(1);
        }
    }

    if let Some(ver) = get_installed_version(&config.binary_path) {
        println!("  [+] Detected Freebuff Version: {}", ver);
    } else {
        println!("  [!] Could not query version directly from binary");
    }

    println!("\n[2/4] Checking Process Safety...");
    let running = process::find_running_freebuff_processes();
    if running.is_empty() {
        println!("  [+] No active Freebuff processes detected.");
    } else {
        println!("  [!] Warning: Found {} running Freebuff process(es):", running.len());
        for p in &running {
            println!("      PID {}: {}", p.pid, p.cmdline);
        }
        if !config.force {
            println!("      Note: --apply will refuse to run while Freebuff is active.");
        }
    }

    println!("\n[3/4] Scanning 13 Advertisement Signatures...");
    let data = match fs::read(&config.binary_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Failed to read binary: {}", e);
            return ExitCode::from(1);
        }
    };

    let scan = patch::scan_binary(&data);
    print_scan_summary(&scan);

    println!("\n[4/4] Checking Local Settings File...");
    match settings::inspect_settings(&config.settings_path) {
        Ok(status) => {
            println!("  [+] Settings file found: {}", config.settings_path.display());
            println!("      adsEnabled:                  {:?}", status.ads_enabled);
            println!("      displayAds:                  {:?}", status.display_ads);
            println!("      displayAdSplashHiddenOn:     {:?}", status.display_ad_splash_hidden_on);
            if status.is_fully_configured {
                println!("      Status: Fully configured for ad removal");
            } else {
                println!("      Status: Needs update (adsEnabled should be false, displayAds empty)");
            }
        }
        Err(e) => {
            println!("  [!] Settings file check notice: {}", e);
        }
    }

    let unpatched_count = scan.iter().filter(|s| s.state == SignatureState::Unpatched).count();
    let patched_count = scan.iter().filter(|s| s.state == SignatureState::AlreadyPatched).count();
    let missing_count = scan.iter().filter(|s| s.state == SignatureState::Missing).count();
    let ambiguous_count = scan.iter().filter(|s| s.state == SignatureState::Ambiguous).count();

    println!("\n------------------------------------------------------------");
    if missing_count > 0 || ambiguous_count > 0 {
        eprintln!("[-] Scan finished with blockers: {} missing, {} ambiguous signatures.", missing_count, ambiguous_count);
        return ExitCode::from(1);
    }

    if unpatched_count == 0 && patched_count == 13 {
        println!("[+] Status: Binary is ALREADY 100% patched!");
    } else {
        println!("[+] Status: Binary is ready for patching ({} unpatched, {} already patched).", unpatched_count, patched_count);
        println!("    Run 'freebuff-patcher --apply' to apply changes safely.");
    }

    ExitCode::SUCCESS
}

fn cmd_apply(config: &Config) -> ExitCode {
    println!("\n[1/5] Safety & Process Verification...");
    if !config.force {
        if let Err(e) = process::check_process_safety() {
            eprintln!("[-] Safety check failed: {}", e);
            eprintln!("    Close Freebuff or use --force to override.");
            return ExitCode::from(1);
        }
    }

    if !config.binary_path.exists() {
        eprintln!("[-] Target binary not found at {}", config.binary_path.display());
        return ExitCode::from(1);
    }

    let backup_mgr = BackupManager::new(&config.binary_path, &config.settings_path);

    println!("\n[2/5] Creating Backups...");
    match backup_mgr.backup_binary() {
        Ok(created) => {
            if created {
                println!("  [+] Binary backup created: {}", backup_mgr.binary_backup_path().display());
            } else {
                println!("  [+] Existing clean binary backup preserved: {}", backup_mgr.binary_backup_path().display());
            }
        }
        Err(e) => {
            eprintln!("[-] Failed to backup binary: {}", e);
            return ExitCode::from(1);
        }
    }

    if config.settings_path.exists() {
        match backup_mgr.backup_settings() {
            Ok(created) => {
                if created {
                    println!("  [+] Settings backup created: {}", backup_mgr.settings_backup_path().display());
                } else {
                    println!("  [+] Existing clean settings backup preserved: {}", backup_mgr.settings_backup_path().display());
                }
            }
            Err(e) => {
                eprintln!("[-] Failed to backup settings: {}", e);
                return ExitCode::from(1);
            }
        }
    }

    println!("\n[3/5] Scanning and Patching Binary in Memory...");
    let mut data = match fs::read(&config.binary_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Failed to read binary: {}", e);
            return ExitCode::from(1);
        }
    };

    let orig_len = data.len();
    match patch::apply_patches_in_memory(&mut data) {
        Ok(applied) => {
            println!("  [+] Successfully applied {} byte patch(es) in memory.", applied);
        }
        Err(e) => {
            eprintln!("[-] Patching aborted: {}", e);
            return ExitCode::from(1);
        }
    }

    if data.len() != orig_len {
        eprintln!("[-] Fatal: Binary size changed during in-memory patching (orig {} vs new {})", orig_len, data.len());
        return ExitCode::from(1);
    }

    println!("\n[4/5] Committing Patched Executable Atomically...");
    if let Err(e) = backup_mgr.write_atomic_binary(&data) {
        eprintln!("[-] Atomic write failed: {}. Attempting rollback...", e);
        let _ = backup_mgr.restore_binary();
        return ExitCode::from(1);
    }

    // Verify immediately after write
    println!("  [+] Verifying written binary...");
    let written_data = match fs::read(&config.binary_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Failed to read back written binary: {}. Rolling back...", e);
            let _ = backup_mgr.restore_binary();
            return ExitCode::from(1);
        }
    };

    let verify_scan = patch::scan_binary(&written_data);
    let all_patched = verify_scan.iter().all(|s| s.state == SignatureState::AlreadyPatched);
    if !all_patched || written_data.len() != orig_len {
        eprintln!("[-] Post-write verification failed! Rolling back from backup...");
        let _ = backup_mgr.restore_binary();
        return ExitCode::from(1);
    }
    println!("  [+] Verification confirmed all 13 patches are in place.");

    println!("\n[5/5] Updating Local Settings...");
    match settings::update_settings_for_ad_removal(&config.settings_path) {
        Ok(date) => {
            println!("  [+] Settings updated:");
            println!("      adsEnabled:              false");
            println!("      displayAds:              []");
            println!("      displayAdSplashHiddenOn: \"{}\"", date);
        }
        Err(e) => {
            eprintln!("  [-] Failed to update settings: {}", e);
            eprintln!("      Binary was patched successfully, but settings require manual update.");
            return ExitCode::from(1);
        }
    }

    // Smoke test terminal
    println!("\n--- Smoke Testing Patched Executable ---");
    smoke_test_binary(&config.binary_path);

    println!("\n============================================================");
    println!("[+] SUCCESS: Freebuff ad patcher completed cleanly.");
    println!("    All 13 advertisement surfaces have been neutralized.");
    println!("    Backups preserved at:");
    println!("      {}", backup_mgr.binary_backup_path().display());
    if backup_mgr.settings_backup_exists() {
        println!("      {}", backup_mgr.settings_backup_path().display());
    }
    println!("============================================================");

    ExitCode::SUCCESS
}

fn cmd_verify(config: &Config) -> ExitCode {
    println!("\n[1/3] Verifying Binary Patches...");
    if !config.binary_path.exists() {
        eprintln!("[-] Executable not found at {}", config.binary_path.display());
        return ExitCode::from(1);
    }

    let data = match fs::read(&config.binary_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[-] Failed to read binary: {}", e);
            return ExitCode::from(1);
        }
    };

    let scan = patch::scan_binary(&data);
    print_scan_summary(&scan);

    let all_patched = scan.iter().all(|s| s.state == SignatureState::AlreadyPatched);
    let any_original = scan.iter().any(|s| s.original_count > 0);

    if !all_patched || any_original {
        eprintln!("\n[-] Verification FAILED: Binary is not fully patched.");
        return ExitCode::from(1);
    }
    println!("\n  [+] Binary verification PASSED (all 13 patches confirmed active).");

    println!("\n[2/3] Verifying Local Settings...");
    match settings::inspect_settings(&config.settings_path) {
        Ok(status) => {
            println!("      adsEnabled:              {:?}", status.ads_enabled);
            println!("      displayAds:              {:?}", status.display_ads);
            println!("      displayAdSplashHiddenOn: {:?}", status.display_ad_splash_hidden_on);

            if status.ads_enabled != Some(false) {
                eprintln!("[-] Settings verification failed: adsEnabled is not false");
                return ExitCode::from(1);
            }
            if status.display_ads.as_ref().map(|v| !v.is_empty()).unwrap_or(true) {
                eprintln!("[-] Settings verification failed: displayAds is not empty list []");
                return ExitCode::from(1);
            }
            if status.display_ad_splash_hidden_on.is_none() {
                eprintln!("[-] Settings verification failed: displayAdSplashHiddenOn is not set");
                return ExitCode::from(1);
            }
            println!("  [+] Settings verification PASSED.");
        }
        Err(e) => {
            eprintln!("[-] Settings verification error: {}", e);
            return ExitCode::from(1);
        }
    }

    println!("\n[3/3] Smoke Testing Freebuff Commands...");
    if !smoke_test_binary(&config.binary_path) {
        eprintln!("[-] Smoke test failed!");
        return ExitCode::from(1);
    }

    println!("\n============================================================");
    println!("[+] VERIFIED: Freebuff executable and settings are fully patched.");
    println!("============================================================");
    ExitCode::SUCCESS
}

fn cmd_restore(config: &Config) -> ExitCode {
    println!("\n[1/3] Safety & Process Verification...");
    if !config.force {
        if let Err(e) = process::check_process_safety() {
            eprintln!("[-] Cannot restore while Freebuff is running: {}", e);
            return ExitCode::from(1);
        }
    }

    let backup_mgr = BackupManager::new(&config.binary_path, &config.settings_path);
    if !backup_mgr.binary_backup_exists() {
        eprintln!("[-] No binary backup found at {}", backup_mgr.binary_backup_path().display());
        return ExitCode::from(1);
    }

    println!("\n[2/3] Restoring Original Binary...");
    if let Err(e) = backup_mgr.restore_binary() {
        eprintln!("[-] Failed to restore binary: {}", e);
        return ExitCode::from(1);
    }
    println!("  [+] Binary restored from {}", backup_mgr.binary_backup_path().display());

    if backup_mgr.settings_backup_exists() {
        match backup_mgr.restore_settings() {
            Ok(true) => println!("  [+] Settings restored from {}", backup_mgr.settings_backup_path().display()),
            Ok(false) => {}
            Err(e) => eprintln!("  [!] Warning: Failed to restore settings: {}", e),
        }
    }

    println!("\n[3/3] Validating Restored Binary...");
    smoke_test_binary(&config.binary_path);

    println!("\n[+] Restoration complete.");
    ExitCode::SUCCESS
}

fn print_scan_summary(scan: &[PatchScanResult]) {
    println!("  ID  | Status         | Surface                 | Name");
    println!("  ----+----------------+-------------------------+--------------------------------");
    for res in scan {
        let status_str = match res.state {
            SignatureState::Unpatched => "UNPATCHED      ",
            SignatureState::AlreadyPatched => "ALREADY PATCHED",
            SignatureState::Missing => "MISSING        ",
            SignatureState::Ambiguous => "AMBIGUOUS      ",
        };
        let symbol = match res.state {
            SignatureState::Unpatched => "[-]",
            SignatureState::AlreadyPatched => "[+]",
            SignatureState::Missing => "[!]",
            SignatureState::Ambiguous => "[?]",
        };
        println!(
            "  {:02}  | {} {} | {:<23} | {}",
            res.patch.id, symbol, status_str, res.patch.surface, res.patch.name
        );
    }
}

fn smoke_test_binary(path: &Path) -> bool {
    let mut ok = true;
    print!("  Testing 'freebuff --version'... ");
    match Command::new(path).arg("--version").output() {
        Ok(out) if out.status.success() => {
            println!("OK ({})", String::from_utf8_lossy(&out.stdout).trim());
        }
        Ok(out) => {
            println!("FAILED (exit code {})", out.status);
            eprintln!("    Stderr: {}", String::from_utf8_lossy(&out.stderr).trim());
            ok = false;
        }
        Err(e) => {
            println!("FAILED ({})", e);
            ok = false;
        }
    }

    print!("  Testing 'freebuff --help'... ");
    match Command::new(path).arg("--help").output() {
        Ok(out) if out.status.success() => {
            println!("OK");
        }
        Ok(out) => {
            println!("FAILED (exit code {})", out.status);
            ok = false;
        }
        Err(e) => {
            println!("FAILED ({})", e);
            ok = false;
        }
    }

    ok
}
