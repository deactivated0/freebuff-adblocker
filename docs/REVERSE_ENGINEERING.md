# Freebuff Terminal: Reverse Engineering & Binary Patching Architecture

This document describes the architectural analysis, binary structures, and patch engineering implemented in `freebuff-adblocker` for the Freebuff Terminal on Linux x86_64.

---

## 1. Executable Format & Runtime Mechanics

### Target Executable
- **Target Path:** `~/.config/manicode/freebuff`
- **Target Architecture:** ELF 64-bit LSB (x86-64)
- **Target Size:** Exactly 138,442,880 bytes
- **Engine:** Standalone Bun runtime embedding a bundled virtual asset filesystem (`bunfs`).

### Bun Executable Constraints
Bun single-file executables combine native machine code with an embedded bundle archive and an end-of-file trailer (`\n---- Bun! ----\n`). The trailer encodes fixed byte offsets into the executable:
- **Size Invariance:** Any shift in file size alters trailer alignment, which breaks bunfs asset lookup and causes Bun to drop to its default command-line interface instead of running the terminal application. All byte replacements must be identical in length to the original signature (`orig.len() == repl.len()`).
- **Syntax Integrity:** Minified JavaScript bundles frequently chain component declarations with comma operators. Modifying functions within expressions requires maintaining syntactically valid JavaScript (e.g., matching function parameters and using balanced dummy expressions) so that the JavaScript engine parses the bundle without syntax errors.

---

## 2. Identified Advertisement Surfaces

Analytical inspection of the bundled code identified 15 discrete surfaces where promotional content, partner deals, or ad-serving network telemetry were integrated.

### Surface Breakdown

1. **Global Ad State (`uJ`):**
   - **Component:** Global settings evaluation.
   - **Function:** Determines whether advertisements should be active based on user subscription status.
   - **Patch Effect:** Evaluates to inactive globally, disabling the base advertising flag.

2. **Deactivation Policy Gate (`yyA`):**
   - **Component:** Settings enforcement logic.
   - **Function:** Rejects user attempts to turn off advertisements while in the free tier.
   - **Patch Effect:** Suppresses the rejection logic, allowing configuration updates to disable ad rendering.

3. **Default Preferences State (`yRA`):**
   - **Component:** Initial settings template.
   - **Function:** Initializes the runtime environment with active ad flags.
   - **Patch Effect:** Sets default runtime state to ad-disabled.

4. **Central Ad Telemetry & Network Dispatcher (`hY`):**
   - **Component:** Network request handler.
   - **Function:** Dispatches outbound HTTP requests to ad servers to retrieve promotional assets.
   - **Patch Effect:** Returns an immediate empty response, eliminating outbound ad-related network traffic.

5. **Remote Policy & Dock Arming (`iaL`):**
   - **Component:** Policy synchronization.
   - **Function:** Fetches remote server configurations for sponsored task docks.
   - **Patch Effect:** Returns a local static control response without initiating network calls.

6. **Promotional Dock UI (`fzL`):**
   - **Component:** Persistent terminal dock banner.
   - **Function:** Renders a visual promotional banner inside the terminal frame.
   - **Patch Effect:** Replaced with a null component, rendering nothing in the UI hierarchy.

7. **Composer & Inline Response Gate (`dRL`):**
   - **Component:** Conversational assistant output filter.
   - **Function:** Injects promotional notices below assistant responses and within the prompt composer.
   - **Patch Effect:** Enforces output suppression regardless of terminal dimensions or operational modes.

8. **Modal Splash Banner (`IcA`):**
   - **Component:** Startup splash dialog.
   - **Function:** Displays modal promotional overlays upon terminal startup.
   - **Patch Effect:** Converted into a no-op null renderer.

9. **Statusline Promotional Display (`obA`):**
   - **Component:** Bottom status bar overlay.
   - **Function:** Displays dynamic sponsored partner messages and links in the terminal statusline.
   - **Patch Effect:** Converted into a no-op null renderer.

10. **Display Ad Active Registry (`vkH`):**
    - **Component:** Surface registration check.
    - **Function:** Verifies whether given ad placements are currently enabled in the client registry.
    - **Patch Effect:** Returns false for all placement queries.

11. **Sponsored Survey Handler (`FA$`):**
    - **Component:** Interactive questionnaire system.
    - **Function:** Injects sponsored survey prompts into the terminal workflow.
    - **Patch Effect:** Returns null immediately, preventing survey instantiation.

12. **Slash Command Menu Recommendations (`r0$`):**
    - **Component:** Command autocomplete menu.
    - **Function:** Injects sponsored partner items into slash command suggestions.
    - **Patch Effect:** Preserves normal autocomplete while omitting injected promotional items.

13. **Sponsored Task Proposals (`BJ`):**
    - **Component:** Task dock classifier.
    - **Function:** Categorizes partner task items for presentation in the workflow dock.
    - **Patch Effect:** Evaluates false for sponsored proposals.

14. **Developer Profile Surveys (`MA$`):**
    - **Component:** Developer profile questionnaire builder (`profile-survey-machine.ts`).
    - **Function:** Builds multi-step developer questionnaires (such as "Who pays for your dev tools?").
    - **Patch Effect:** Returns null immediately, preventing developer survey object instantiation and blocking state transition to asking mode.

15. **Survey Box UI Component (`_1$`):**
    - **Component:** Survey box presentation container (`profile-survey-box.tsx`).
    - **Function:** Renders the survey questionnaire box with choice options and step indicators in the terminal layout.
    - **Patch Effect:** Evaluates to a null component (`()=>null`), preventing visual rendering of any survey containers.

---

## 3. Local Settings Synchronization

Freebuff persists user settings in `~/.config/manicode/settings.json`. The patcher updates this configuration atomically:
- `adsEnabled: false`
- `displayAds: []`
- `displayAdSplashHiddenOn: "<local-date>"`

All unrelated user settings, configuration options, themes, and identifiers are strictly preserved.

---

## 4. Safety & Rollback Architecture

- **Active Process Detection:** Checks `/proc` to guarantee Freebuff is not running before modifying the executable.
- **Pre-Modification Backup:** Stores byte-exact backups (`freebuff.bak` and `settings.json.bak`) prior to making changes. Clean backups are never overwritten by subsequent runs.
- **In-Memory Modification:** Validates all signatures, computes replacements, and checks buffer length before writing to disk.
- **Atomic Replacement:** Writes the new executable to a temporary file, sets execution permissions (`0755`), and replaces the original via an atomic rename operation.
- **Automated Rollback:** Reads back and verifies all 15 signatures post-write; automatically restores original files if any verification check fails.
