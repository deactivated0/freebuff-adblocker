# Freebuff Adblocker (`freebuff-adblocker`)

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20x86__64-lightgrey.svg)]()
[![Dependencies](https://img.shields.io/badge/dependencies-0%20(std%20only)-brightgreen.svg)]()
[![Binary Size](https://img.shields.io/badge/release%20binary-~430%20KB-success.svg)]()

> A 100% native Rust binary patcher and ad-removal utility for **Freebuff Terminal** on Linux x86_64.
> Eliminates all 13 advertisement surfaces, suppresses outbound ad network requests, and synchronizes local preferences with zero external dependencies.

---

## Overview

Freebuff Terminal incorporates promotional banners, modal splash dialogs, statusline partner links, sponsored task proposals, and ad-related network telemetry. 

`freebuff-adblocker` is a lightweight, zero-dependency Rust CLI utility engineered to safely and reversibly disable all advertisement functionality within the local Freebuff standalone executable (`~/.config/manicode/freebuff`) and user configuration (`~/.config/manicode/settings.json`).

### Why `freebuff-adblocker`?

- **100% Native Rust:** Built with Rust Edition 2024 using only the standard library. No Python runtime, Node.js, or third-party crates required.
- **Microscopic Footprint:** Produces a stripped ~430 KB release binary with instant startup and fast scanning (<3 seconds across the 138 MB executable).
- **Zero Outbound Ad Telemetry:** Short-circuits network dispatchers for ad retrieval and tracking.
- **Bun Runtime Safe:** Freebuff is packaged as a standalone [Bun](https://bun.sh) binary with a fixed file trailer. `freebuff-adblocker` uses byte-exact in-place patching to preserve file size (exactly 138,442,880 bytes), maintaining runtime integrity.
- **Safety First:** Defaults to dry-run mode, refuses to modify binaries while Freebuff is running, generates clean backups, and supports one-command rollback (`--restore`).
- **Preserves Normal Terminal Workflows:** Retains full slash-command autocompletion, terminal themes, settings, and assistant capabilities.

---

## Neutralized Advertisement Surfaces

`freebuff-adblocker` comprehensively disables all 13 promotional and telemetry surfaces:

| # | Surface | Scope | Neutralization Effect |
|---|---|---|---|
| **01** | Global Ad State | Settings | Forces global `adsEnabled` logic to evaluate to false |
| **02** | Deactivation Gate | Policy | Bypasses policy check preventing ad deactivation |
| **03** | Default Settings | Defaults | Sets default runtime configuration to ad-disabled |
| **04** | Central Ad Dispatcher | Network | Outbound ad retrieval requests return null immediately |
| **05** | Remote Policy Fetch | Network | Ad policy queries return local control without network calls |
| **06** | Promotional Dock UI | Terminal UI | Replaces visual promotional banner component with null renderer |
| **07** | Composer & Inline Gate | Assistant | Suppresses promotional notices in composer and responses |
| **08** | Modal Splash Banner | Startup UI | Replaces startup modal dialog with null renderer |
| **09** | Statusline Ad Display | Status Bar | Removes partner promotions and deal links from bottom bar |
| **10** | Display Ad Registry | Client Core | Forces ad surface active check to return false for all placements |
| **11** | Sponsored Surveys | Questionnaire | Bypasses and drops interactive sponsored survey prompts |
| **12** | Slash Recommendations | Autocomplete | Removes injected partner promotions while preserving slash commands |
| **13** | Sponsored Task Docks | Workflow Dock | Classifies sponsored proposals as false to prevent dock takeovers |

For full architectural details and technical analysis, see the [Reverse Engineering Documentation](docs/REVERSE_ENGINEERING.md).

---

## Local Settings Synchronization

In addition to binary patching, the utility updates `~/.config/manicode/settings.json` atomically:

```json
{
  "adsEnabled": false,
  "displayAds": [],
  "displayAdSplashHiddenOn": "<current-local-date>"
}
```

All existing user keys, themes, and configuration parameters are strictly preserved.

---

## Quick Start

### Prerequisites
- Linux x86_64
- Rust toolchain (1.85+ / Edition 2024)

### Building from Source

```bash
git clone https://github.com/deactivated0/freebuff-adblocker.git
cd freebuff-adblocker
cargo build --release
```

The optimized binary will be located at `target/release/freebuff-patcher`.

---

## CLI Usage

The patcher provides a simple, diagnostic-rich command line interface:

```bash
freebuff-patcher [OPTIONS] [COMMAND]
```

### Commands

#### 1. Dry-Run Inspection (`--check`)
Inspects the executable format, checks running processes, scans for all 13 signatures, and verifies configuration files without modifying anything:
```bash
./target/release/freebuff-patcher --check
```

#### 2. Apply Patches (`--apply`)
Backs up original files, applies all 13 byte replacements in memory, atomically replaces the binary, updates settings, and executes terminal smoke tests:
```bash
./target/release/freebuff-patcher --apply
```

#### 3. Verify Active Patches (`--verify`)
Validates that all 13 patches and settings modifications are currently in effect:
```bash
./target/release/freebuff-patcher --verify
```

#### 4. Restore Original Files (`--restore`)
Rolls back both the executable and settings to their pre-patched state from backups:
```bash
./target/release/freebuff-patcher --restore
```

#### 5. Help (`--help`)
Displays command usage, option flags, and exit codes:
```bash
./target/release/freebuff-patcher --help
```

### Options

- `--target <PATH>`: Custom path to `freebuff` executable (default: `~/.config/manicode/freebuff`).
- `--settings <PATH>`: Custom path to `settings.json` (default: `~/.config/manicode/settings.json`).
- `--force`: Bypass active process check.

---

## Safety & Rollback Architecture

- **Active Process Detection:** Inspects `/proc` and refuses to patch while `freebuff` is running, avoiding file write conflicts.
- **Clean Backups:** Creates `freebuff.bak` and `settings.json.bak` on initial apply. Backups are never overwritten on subsequent runs.
- **Atomic File Writing:** Writes patched bytes to a temporary file on the same filesystem and swaps it into place via atomic rename, preserving executable permissions (`0755`).
- **Self-Verifying Rollback:** Reads back and verifies the written binary; if any byte signature or size verification fails, it rolls back automatically.
- **Manual Rollback:** Roll back at any time using `--restore`.

---

## Testing

An automated integration and unit test suite is included:

```bash
cargo test
```

Tests cover:
- Exact byte-length equality across all 13 patches.
- Detection and abortion on missing signatures.
- Detection and abortion on ambiguous/duplicate signatures.
- Idempotence verification (repeated patch applications).
- Settings preservation and atomic JSON updates.
- Backup creation, restoration, and file permission checks.
- ELF header validation.

---

## License

This project is licensed under the [MIT License](LICENSE).
