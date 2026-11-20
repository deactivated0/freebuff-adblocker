---
name: freebuff-adblocker-maintainer
description: Comprehensive operational guide and maintenance manual for AI agents maintaining, updating, and extending the freebuff-adblocker codebase.
---

# Freebuff Adblocker Maintenance Skill for AI Agents

This guide provides instructions, architectural constraints, and standard workflows for AI agents tasked with maintaining, updating, or extending `freebuff-adblocker`.

---

## 1. Core Principles & Hard Invariants

When modifying or updating this project, every AI agent must strictly follow these rules:

1. **Zero External Dependencies:**
   - The project uses **100% Rust (Edition 2024)** standard library (`std`) only.
   - Do **not** add third-party crates (e.g., `serde`, `clap`, `regex`, `tokio`). The binary must remain under 500 KB, fast-compiling, and self-contained.

2. **Byte-Exact Replacement Invariant:**
   - Freebuff is a standalone Bun binary with an end-of-file trailer (`\n---- Bun! ----\n`) pointing to fixed offsets.
   - Any length shift (even ±1 byte) invalidates trailer alignment and breaks runtime bundle extraction.
   - Every replacement slice in `ALL_PATCHES` **must** satisfy:
     ```rust
     assert_eq!(patch.original.len(), patch.replacement.len());
     ```

3. **AST Syntax Integrity in Minified JS:**
   - The embedded bunfs bundle minifies statements with comma operators (`let A=...,B=...;`).
   - Never replace comma-chained statements with bare `return` statements (`return null;,E=...` is a syntax error).
   - Use dummy functional expressions (e.g., `()=>null,dummy=(...)=>`) or closed function declarations with comment padding (`/*...*/`).

4. **Atomic I/O and File Permissions:**
   - Never write directly to the live executable.
   - Always write to a temporary file on the same filesystem, set executable permissions (`0755`), and swap via `std::fs::rename`.
   - Never overwrite existing clean backups (`freebuff.bak` and `settings.json.bak`).

5. **No Hardcoded Paths or Personal Identifiers:**
   - Never hardcode user names, home paths, or machine-specific directories.
   - Always query `std::env::var("HOME")`.

---

## 2. Codebase Architecture

```
freebuff-patcher/
├── Cargo.toml                  # Edition 2024, release profile (opt-level = "z", LTO, abort panic, strip)
├── README.md                   # Public documentation and usage guide
├── SKILL.md                    # This maintenance manual for AI agents
├── docs/
│   └── REVERSE_ENGINEERING.md  # Detailed technical breakdown of all 15 ad and survey surfaces
├── src/
│   ├── lib.rs                  # Library entrypoint, ELF format validation, version detection
│   ├── main.rs                 # CLI dispatch (--check, --apply, --verify, --restore, --help)
│   ├── patch.rs                # ALL_PATCHES list, vectorized scanner, in-memory patch applier
│   ├── settings.rs             # Zero-dependency JSON parser/serializer, atomic settings updater
│   ├── process.rs              # /proc process inspector & pgrep fallback for safety check
│   └── backup.rs               # Atomic file replacer, backup creator, rollback manager
└── tests/
    └── integration_tests.rs    # Automated test suite covering patches, settings, and edge cases
```

---

## 3. Routine Maintenance Workflows

### Workflow A: Verifying Code Quality & Tests
Run after every change:
```bash
# 1. Run full test suite
cargo test

# 2. Build release binary and verify output size
cargo build --release
ls -lh target/release/freebuff-patcher

# 3. Verify CLI help output
./target/release/freebuff-patcher --help
```

### Workflow B: Handling a New Freebuff Version
When a new Freebuff release is detected:
1. **Dry-Run Inspection:**
   ```bash
   ./target/release/freebuff-patcher --check
   ```
2. **Examine Scan Output:**
   - If all 15 signatures show `UNPATCHED`, the existing signatures are 100% compatible. Proceed with `--apply`.
   - If any signature shows `MISSING` or `AMBIGUOUS`:
     - Inspect the new binary bundle to locate the modified variable or function names for that specific ad surface.
     - Extract a unique signature with sufficient surrounding context.
     - Craft an exact-length replacement matching the semantics defined in [docs/REVERSE_ENGINEERING.md](docs/REVERSE_ENGINEERING.md).
     - Update the corresponding entry in `src/patch.rs`.
     - Add/update unit tests in `tests/integration_tests.rs`.
     - Re-test with `cargo test` and verify that the patched binary boots cleanly with `freebuff --version` and `freebuff --help`.

### Workflow C: Updating Settings Schema
If Freebuff introduces new ad-related flags in `~/.config/manicode/settings.json`:
1. Check `src/settings.rs` in `update_settings_for_ad_removal`.
2. Add the new settings key using `val.set("newKey", JsonValue::Bool(false))` or appropriate type.
3. Update `inspect_settings` to verify the new key during `--verify`.
4. Update `tests/integration_tests.rs` to validate JSON serialization and key preservation.

---

## 4. Testing Checklist for AI Contributors

Before committing and pushing any changes:

- [ ] `cargo test` passes with 0 failures and 0 warnings.
- [ ] `cargo build --release` produces zero compiler warnings.
- [ ] Release binary size remains under 500 KB.
- [ ] `ALL_PATCHES` length invariance test passes (`orig.len() == repl.len()`).
- [ ] `git status` shows no stray files, binaries, or credentials staged.
- [ ] Documentation in `README.md` and `docs/REVERSE_ENGINEERING.md` matches code state.
