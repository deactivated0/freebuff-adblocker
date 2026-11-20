use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use freebuff_patcher::backup::BackupManager;
use freebuff_patcher::patch::{self, SignatureState, ALL_PATCHES};
use freebuff_patcher::settings::{self, JsonParser, JsonValue};
use freebuff_patcher::check_elf_format;

fn temp_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("freebuff_test_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create temp test dir");
    dir
}

#[test]
fn test_all_15_patches_exact_length_and_validity() {
    assert_eq!(ALL_PATCHES.len(), 15);
    for (i, p) in ALL_PATCHES.iter().enumerate() {
        assert_eq!(p.id, i + 1, "Patch ID should match 1-based index");
        assert_eq!(
            p.original.len(),
            p.replacement.len(),
            "Patch #{} ({}) original and replacement must be identical length! (orig: {}, repl: {})",
            p.id,
            p.name,
            p.original.len(),
            p.replacement.len()
        );
        assert_ne!(
            p.original, p.replacement,
            "Patch #{} ({}) replacement must not be identical to original",
            p.id, p.name
        );
        // Verify both original and replacement are valid UTF-8
        std::str::from_utf8(p.original).expect("Original signature must be valid UTF-8");
        std::str::from_utf8(p.replacement).expect("Replacement signature must be valid UTF-8");
    }
}

#[test]
fn test_missing_signature_aborts_patching() {
    // Buffer with only 12 of the 13 signatures (omitting patch #5 iaL)
    let mut buf = Vec::new();
    for patch in &ALL_PATCHES {
        if patch.id == 5 {
            buf.extend_from_slice(b"SOMETHING_ELSE_ENTIRELY;");
            continue;
        }
        buf.extend_from_slice(patch.original);
        buf.extend_from_slice(b";\n");
    }

    let scan = patch::scan_binary(&buf);
    assert_eq!(scan.len(), 15);
    let patch5_scan = scan.iter().find(|s| s.patch.id == 5).unwrap();
    assert_eq!(patch5_scan.state, SignatureState::Missing);

    let err = patch::apply_patches_in_memory(&mut buf).unwrap_err();
    assert!(
        err.contains("iaL_ad_policy_fetch") || err.contains("#5"),
        "Error message should mention missing patch #5: {}",
        err
    );
}

#[test]
fn test_ambiguous_signature_aborts_patching() {
    // Buffer with patch #1 duplicated twice
    let mut buf = Vec::new();
    for patch in &ALL_PATCHES {
        buf.extend_from_slice(patch.original);
        buf.extend_from_slice(b";\n");
        if patch.id == 1 {
            // Duplicate original #1
            buf.extend_from_slice(patch.original);
            buf.extend_from_slice(b";\n");
        }
    }

    let scan = patch::scan_binary(&buf);
    let patch1_scan = scan.iter().find(|s| s.patch.id == 1).unwrap();
    assert_eq!(patch1_scan.state, SignatureState::Ambiguous);
    assert_eq!(patch1_scan.original_count, 2);

    let err = patch::apply_patches_in_memory(&mut buf).unwrap_err();
    assert!(
        err.contains("ambiguous") && err.contains("#1"),
        "Error message should report ambiguous patch #1: {}",
        err
    );
}

#[test]
fn test_idempotent_patching_and_detection() {
    let mut buf = Vec::new();
    for patch in &ALL_PATCHES {
        buf.extend_from_slice(b"/* start */ ");
        buf.extend_from_slice(patch.original);
        buf.extend_from_slice(b" /* end */\n");
    }

    let orig_len = buf.len();

    // 1st scan: all unpatched
    let scan1 = patch::scan_binary(&buf);
    for res in &scan1 {
        assert_eq!(res.state, SignatureState::Unpatched);
        assert_eq!(res.original_count, 1);
        assert_eq!(res.replacement_count, 0);
    }

    // Apply patches
    let applied = patch::apply_patches_in_memory(&mut buf).expect("first apply must succeed");
    assert_eq!(applied, 15);
    assert_eq!(buf.len(), orig_len, "Binary length must not change");

    // 2nd scan: all already patched
    let scan2 = patch::scan_binary(&buf);
    for res in &scan2 {
        assert_eq!(res.state, SignatureState::AlreadyPatched);
        assert_eq!(res.original_count, 0);
        assert_eq!(res.replacement_count, 1);
    }

    // 2nd apply: idempotent, 0 applied
    let reapplied = patch::apply_patches_in_memory(&mut buf).expect("second apply must succeed");
    assert_eq!(reapplied, 0);
    assert_eq!(buf.len(), orig_len);
}

#[test]
fn test_settings_preservation_and_update() {
    let dir = temp_test_dir("settings");
    let settings_path = dir.join("settings.json");

    let initial_json = r#"{
  "theme": "matrix-green",
  "fontSize": 15,
  "adsEnabled": true,
  "displayAds": ["banner", "dock", "splash"],
  "customKey": {
    "nestedKey": "keepMeIntact"
  }
}"#;
    fs::write(&settings_path, initial_json).unwrap();

    let initial_status = settings::inspect_settings(&settings_path).unwrap();
    assert_eq!(initial_status.ads_enabled, Some(true));
    assert_eq!(initial_status.display_ads.unwrap().len(), 3);
    assert_eq!(initial_status.is_fully_configured, false);

    let date_set = settings::update_settings_for_ad_removal(&settings_path).unwrap();
    assert_eq!(date_set.len(), 10); // YYYY-MM-DD

    let updated_status = settings::inspect_settings(&settings_path).unwrap();
    assert_eq!(updated_status.ads_enabled, Some(false));
    assert_eq!(updated_status.display_ads.unwrap(), Vec::<String>::new());
    assert_eq!(updated_status.display_ad_splash_hidden_on, Some(date_set));
    assert!(updated_status.is_fully_configured);

    // Verify other user settings were strictly preserved
    let updated_content = fs::read_to_string(&settings_path).unwrap();
    let mut parser = JsonParser::new(&updated_content);
    let root = parser.parse().unwrap();

    assert_eq!(root.get("theme"), Some(&JsonValue::String("matrix-green".to_string())));
    assert_eq!(root.get("fontSize"), Some(&JsonValue::Number(15.0)));
    let custom = root.get("customKey").unwrap();
    assert_eq!(custom.get("nestedKey"), Some(&JsonValue::String("keepMeIntact".to_string())));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_backup_and_restore_operations() {
    let dir = temp_test_dir("backup");
    let binary_path = dir.join("freebuff");
    let settings_path = dir.join("settings.json");

    // Create synthetic binary with ELF header
    let mut bin_data = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00ORIGINAL_BIN_CONTENT".to_vec();
    fs::write(&binary_path, &bin_data).unwrap();
    fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755)).unwrap();

    fs::write(&settings_path, b"{\"adsEnabled\": true}").unwrap();

    let bm = BackupManager::new(&binary_path, &settings_path);
    assert!(!bm.binary_backup_exists());
    assert!(!bm.settings_backup_exists());

    // 1. First backup creates files
    assert!(bm.backup_binary().unwrap());
    assert!(bm.backup_settings().unwrap());
    assert!(bm.binary_backup_exists());
    assert!(bm.settings_backup_exists());

    // Check backup permissions
    let bak_mode = fs::metadata(bm.binary_backup_path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(bak_mode, 0o755);

    // 2. Second backup does NOT overwrite existing backup
    assert!(!bm.backup_binary().unwrap());
    assert!(!bm.backup_settings().unwrap());

    // 3. Modify live binary and settings
    bin_data.extend_from_slice(b"_MODIFIED");
    bm.write_atomic_binary(&bin_data).unwrap();
    fs::write(&settings_path, b"{\"adsEnabled\": false}").unwrap();

    let cur_bin = fs::read(&binary_path).unwrap();
    assert!(cur_bin.ends_with(b"_MODIFIED"));

    // 4. Restore
    bm.restore_binary().unwrap();
    bm.restore_settings().unwrap();

    let restored_bin = fs::read(&binary_path).unwrap();
    assert_eq!(restored_bin, b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00ORIGINAL_BIN_CONTENT");
    let restored_mode = fs::metadata(&binary_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(restored_mode, 0o755);

    let restored_settings = fs::read_to_string(&settings_path).unwrap();
    assert_eq!(restored_settings, "{\"adsEnabled\": true}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_elf_validation() {
    let dir = temp_test_dir("elf");
    let elf_file = dir.join("valid_elf");
    let fake_file = dir.join("fake_elf");
    let short_file = dir.join("short_file");

    // 64-bit Little Endian ELF
    let valid_elf_bytes = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    fs::write(&elf_file, valid_elf_bytes).unwrap();
    let res = check_elf_format(&elf_file).unwrap();
    assert!(res.contains("ELF 64-bit LSB"));

    // Windows PE / random file
    fs::write(&fake_file, b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00").unwrap();
    assert!(check_elf_format(&fake_file).is_err());

    // Truncated file (<16 bytes)
    fs::write(&short_file, b"\x7fELF").unwrap();
    assert!(check_elf_format(&short_file).is_err());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_json_parser_robustness() {
    let json_text = r#"{
        "string": "hello \"world\"\n\t\u0041",
        "number_int": 42,
        "number_float": -123.456,
        "bool_true": true,
        "bool_false": false,
        "null_val": null,
        "empty_arr": [],
        "arr": [1, "two", false],
        "nested": {
            "inner": "value"
        }
    }"#;

    let mut parser = JsonParser::new(json_text);
    let val = parser.parse().expect("parsing must succeed");

    assert_eq!(val.get("string"), Some(&JsonValue::String("hello \"world\"\n\tA".to_string())));
    assert_eq!(val.get("number_int"), Some(&JsonValue::Number(42.0)));
    assert_eq!(val.get("number_float"), Some(&JsonValue::Number(-123.456)));
    assert_eq!(val.get("bool_true"), Some(&JsonValue::Bool(true)));
    assert_eq!(val.get("bool_false"), Some(&JsonValue::Bool(false)));
    assert_eq!(val.get("null_val"), Some(&JsonValue::Null));
    assert_eq!(val.get("empty_arr"), Some(&JsonValue::Array(Vec::new())));

    if let Some(JsonValue::Array(items)) = val.get("arr") {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], JsonValue::Number(1.0));
        assert_eq!(items[1], JsonValue::String("two".to_string()));
        assert_eq!(items[2], JsonValue::Bool(false));
    } else {
        panic!("expected array for 'arr'");
    }

    let nested = val.get("nested").expect("expected nested object");
    assert_eq!(nested.get("inner"), Some(&JsonValue::String("value".to_string())));
}
