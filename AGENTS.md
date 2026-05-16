# AGENTS.md

Telegraph style. Root rules only. Read scoped `AGENTS.md` files before subtree work.

## Start

- Repo: `https://github.com/xieyangsp/meiqiu-dict`
- Path references: repo-root relative, e.g. `src-tauri/src/tray.rs:42`. No absolute paths, no `C:\...`, no `~/`.
- High-confidence answers only: before diagnosing or fixing, read the source, related tests, and dependency docs/types. Do not guess Tauri 2, Vue 3, Tailwind, or rusqlite APIs from memory.
- Tauri 2 differs heavily from v1. Never copy v1 snippets blindly. For shortcut, tray, window, and path APIs, verify against the current `tauri 2.2` and `tauri-plugin-*` source or official docs.
- Missing deps: `pnpm install`, retry once, then report the first actionable error.
- All code, comments, and documents are English. End-user-visible UI strings stay in Chinese (the product targets Chinese readers).
- No emoji unless the user asks for them.
- User style: minimal surface area; Occam's razor; clear module boundaries. Do only what was asked or what is clearly required.

## Map

```
meiqiu-dict/
├── index.html  package.json  vite.config.ts  tsconfig.json
├── tailwind.config.ts  postcss.config.js  .gitignore
├── scripts/                            # Node build scripts (icons, dict)
├── src-ui/                             # Frontend: Vue 3 + TS + Tailwind
│   ├── entry.ts                        # Single JS entry; mounts App by ?win=
│   ├── apps/{MainApp,FloaterApp,PopupApp}.vue
│   ├── features/{search,entry,settings}/   # Feature pages
│   ├── shared/{ui/,ipc.ts,types.ts,store.ts}
│   └── assets/  env.d.ts
└── src-tauri/                          # Backend: Rust 2024 + Tauri 2.2
    ├── Cargo.toml  build.rs  tauri.conf.json
    ├── capabilities/                   # Tauri 2 capability files
    ├── icons/  resources/              # Icons + bundled ECDICT db
    └── src/{main,lib,error,state,config,dict,capture,selection,hotkey,tray,window,commands}.rs
```

- Single HTML: `index.html` is shared by every window; the right Vue app is chosen by the query string `?win=main|floater|popup`.
- Window labels must match `src-tauri/capabilities/default.json -> windows`.
- Planned but not yet present: `src-tauri/src/tts.rs` (lands with the TTS milestone).

## Architecture

- Module tiers (only `lib.rs` wires them together):
  - **Infrastructure** (`state`, `error`, `config`): anyone may import.
  - **Utility** (`selection`, `window`): pure helpers; anyone may import. New utility modules must stay pure (no `AppState`, no `AppHandle`, except as parameters).
  - **Business** (`dict`, `capture`, `hotkey`, `tray`, `tts`): **do not import each other**. They communicate through `AppState` and Tauri events.
  - **IPC boundary** (`commands`): the only Rust code the frontend reaches via `invoke`. It is allowed to call into business modules; nothing imports `commands`.
  - **Assembly** (`lib`, `main`): the only place that registers plugins, builds `AppState`, installs the tray, registers the hotkey, and exposes commands. `main.rs` only calls `lib::run()`.
- Pattern: pure function + thin Tauri adapter. Examples: `dict::lookup_conn` (pure) / `dict::lookup` (pool adapter); `window::clamp_rect` (pure) / `window::clamp_to_monitor` (Tauri adapter). Unit-test the pure half; the adapter stays thin.
- Errors: business code returns `AppError` (`src-tauri/src/error.rs`). `commands.rs` propagates `AppResult<T>` to the frontend with `?` (`AppError` already implements `Serialize`).
- Config: `config.rs` is the only place that reads and writes `%APPDATA%\com.meiqiu.dict\config.json`. Other modules take snapshots via `AppState::config()`.
- Capture pipeline is strictly one-way: `mouseup -> back up clipboard -> simulate Ctrl+C -> read -> restore -> floater -> click -> popup`. Any failed step must restore the clipboard and reset state.
- Multi-language support keeps **only three cheap seams**, never a full abstraction:
  1. `DictEntry.lang_pair: &str`
  2. SQLite columns `lang_src TEXT, lang_tgt TEXT`
  3. `selection.rs::is_acceptable_selection()`
- Frontend shared layer `src-ui/shared/`: all IPC goes through `shared/ipc.ts`; components must not call `invoke()` directly. Type definitions live in `shared/types.ts` and match Rust `serde` structs one-to-one.
- Window strategy: main, floater, and popup are independent `WebviewWindow` instances. No iframes, no webview reuse across logical windows.

## Commands

- Runtime: Node >= 20, pnpm >= 9, Rust stable >= 1.85 (edition 2024). Windows 11 is the primary target.
- Install: `pnpm install`.
- Dev: `pnpm tauri dev` (starts Vite and Cargo automatically).
- Build: `pnpm tauri build` (NSIS installer; see `tauri.conf.json -> bundle.targets`).
- Icons: `pnpm icons` regenerates app and tray PNG/ICO outputs from `src-tauri/icons/source-active.png` and `source-idle.png`.
- Dictionary data: put the ECDICT csv at `scripts/data/ecdict.csv` (https://github.com/skywind3000/ECDICT), then `pnpm dict` produces `src-tauri/resources/ecdict.db` (~45 MiB, ~770k rows). Both files are gitignored. `pnpm dict` must run once before `pnpm tauri build`, otherwise the bundler fails because `bundle.resources` points at a missing file. `pnpm tauri dev` tolerates a missing db: the app logs a warning and `dict_lookup` returns an error, the rest of the app still runs.
- Frontend typecheck: `pnpm build` runs `vue-tsc --noEmit`. Standalone: `pnpm exec vue-tsc --noEmit`.
- Rust checks: `cd src-tauri && cargo check`; tests: `cargo test`; format: `cargo fmt`; lint: `cargo clippy --all-targets -- -D warnings`.

## Code

- Rust: edition 2024, `rust-version = "1.85"`. `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` belongs only in `main.rs`.
- Errors: business code returns `AppResult<T>` (`crate::error::AppResult`). No `unwrap()` or `expect()` outside the assembly path in `lib.rs::run`, and there only when failure should crash the process.
- Logging: `tauri-plugin-log` plus the `log` facade. Do not mix in `tracing`, `println!`, or `eprintln!`.
- Concurrency: shared state uses `parking_lot::{Mutex, RwLock}`. Do not mix with `std::sync` equivalents.
- Resources: small static assets (icons) ship through `include_bytes!`. Large data (ECDICT db) ships via `tauri.conf.json -> bundle.resources` and is resolved at runtime with `app.path().resolve("resources/ecdict.db", BaseDirectory::Resource)`. The same call returns the dev copy under `target/<profile>/resources/` and the installed copy under `<Install>/resources/`.
- TypeScript: strict everywhere. Frontend types come from `shared/types.ts` only; do not redeclare Rust structs inside components.
- Vue: `<script setup lang="ts">` only. No Options API.
- Styling: Tailwind utilities by default. For richer components, copy shadcn-vue primitives into `src-ui/shared/ui/` on demand. Never install the full shadcn-vue package.
- Naming: English identifiers everywhere. Product is **MeiQiuDict / 煤球词典**. Bundle id `com.meiqiu.dict`, crate `meiqiu-dict`, lib `meiqiu_dict_lib`.
- Never put milestone identifiers (`PR-1`, `M1`, `Step 3`, `Phase 1`, etc.) in code, comments, tests, or commit messages.
- Language rule: code, comments, and developer-facing log/error strings are English. Only end-user-visible UI strings (window titles, tray menu labels, tooltips, button text) stay in Chinese.

### Comments

Default to self-documenting code. A comment is a code smell: before writing one, try a better name, a smaller function, a named constant, or a type. The rules below apply to every language in the repo (Rust, TS, Vue, mjs scripts, JSON-with-comments where allowed).

- **Hard cap: 3 lines per comment block**, including `///` / `/** */` doc comments and Vue `<!-- -->`. If you need more, the code or the names are wrong, not the comment budget.
- **Explain WHY, not WHAT.** The code already says what. A comment earns its place only when a reader cannot infer *why* from names and types: a non-obvious tradeoff, an OS quirk, a spec citation, a workaround for a known bug.
- **No doc comments that paraphrase the signature.** `/// Look up a word.` on `fn lookup(word: &str) -> Option<DictEntry>` is noise. Delete it.
- **No file/module banner comments.** Module roles are documented in `AGENTS.md`. Do not put a prose block above `mod foo;` or at the top of `foo.rs`, `foo.ts`, or `foo.vue`. Doc comments belong on the public item.
- **No cross-references from code to AGENTS.md, AI memory, PRs, issues, or milestones.** Source must read standalone. Cite external facts with a terse link only: `// Win32 SendInput: <msdn-url>`.
- **No commented-out code.** Delete it; git remembers.
- **`TODO` / `FIXME` require an owner and a tracker link**, e.g. `// TODO(xieyang): #42 multi-monitor DPI`. Bare `TODO:` is forbidden.
- **No design history, no "previously this was X", no phase or milestone tags.**
- **No emoji, no ASCII art, no boxed dividers** (`// ==========`).
- **Formatting.** One blank line above a comment block, none below; comment sits immediately on top of the code it describes. Inline `// ...` trailing comments are allowed but counted against the 3-line cap when stacked.
- **Tests are documentation.** Prefer a named test case (`fn rejects_empty_selection()`) over a paragraph explaining tricky logic.

## Tests

- Rust unit tests: `#[cfg(test)] mod tests` colocated with the source. Keep them local; avoid cross-module hops.
- Integration tests: `src-tauri/tests/*.rs` for pure logic (dictionary lookup, clipboard restore ordering). UI and capture paths that need Windows APIs use manual verification scripts and stay out of CI.
- Do not mock the Tauri runtime. Anything that needs a Tauri context is split into a pure function plus a thin adapter; only the pure function is unit-tested.
- Coverage policy: optimize for regression prevention, not coverage percentage.
- Test names describe behavior. No phase or milestone tags (see `Code`).

## Git

- Commits: `git commit -m "..."`; concise, verb-first, English or Chinese as needed.
- **Never force-push to an already-pushed PR branch.** No `commit --amend` plus `--force-with-lease` for review fix-ups; always add follow-up commits.
- Force-push is allowed only for: an explicit rebase onto fresh main, or branches that exist locally and were never pushed.
- Default flow: `git commit` then `git push`. Do not rebase main unless the user explicitly asks to sync.
- User says "commit": current-conversation changes only. "commit all": every pending change, grouped sensibly. "push": optionally `git pull --rebase` first.
- Do not delete or rename unexpected files. Ask if it blocks progress; otherwise leave them alone.

### PR titles

PRs squash-merge into `main`, so the PR title becomes the commit message on `main`. Follow Conventional Commits:

- Format: `<type>(<scope>)?: <subject>`
- Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `build`, `ci`, `perf`, `style`
- Scope is optional and names a module (`tray`, `hotkey`, `dict`, `capture`, `agents`, etc.)
- Subject: lowercase except proper nouns (`Tauri`, `ECDICT`, `煤球` etc.); imperative mood (`add`, not `added`/`adds`); no trailing period
- Length: keep the whole title <= 72 characters
- Breaking changes: append `!` after type/scope, e.g. `feat(config)!: rename autostart key`
- Never embed phase or milestone identifiers (`PR-1`, `M2`, `Phase 0`); see `Code`
- No emoji

Examples:

```
feat(tray): show 煤球 sleeping when capture disabled
fix(hotkey): restore clipboard if Ctrl+C simulation fails
chore: bump tauri to 2.3
refactor(state): replace RwLock with Mutex
docs(agents): add PR title convention
```

## Docs / Memory

- Long-lived planning and historical decisions live in **AI memory** (`/memories/repo/meiqiu-dict.md`), not in repo markdown. Do not commit `PLAN.md`, `ROADMAP.md`, or change-log style files unless explicitly requested.
- READMEs follow a three-section shape when needed: what it is, how to use it, how to develop on it. No phase numbers.
- Behavior or API changes update the matching doc. Pure internal refactors do not require doc churn.

## Roadmap (reminder; never embed in code)

1. Tray and hotkey skeleton.
2. Dictionary lookup: ECDICT -> SQLite, rusqlite plus an r2d2 pool.
3. Capture pipeline: rdev mouse hook, arboard/enigo clipboard cycle, floater window.
4. TTS: Windows SAPI or Edge TTS (decision pending).
5. System integration: autostart, single-instance, settings UI.
6. Packaging: NSIS installer, optional code signing.

Detailed decisions live in `/memories/repo/meiqiu-dict.md`.

## Security / Privacy

- Never commit real user data, captured clipboard samples, or unredacted lookup logs.
- The config file `%APPDATA%\com.meiqiu.dict\config.json` stores non-sensitive settings only. If credentials are ever needed, route them through the OS keyring; do not put them in plaintext JSON.
- The product is offline. Any future online fallback (translation API, etc.) must default off and require an explicit opt-in in the settings UI.

## Footguns

- Tauri 2 `Manager::tray_by_id` needs the `tray-icon` feature. After changing features, run `cargo clean` once.
- `tauri-plugin-global-shortcut` accelerator strings are case sensitive (`CommandOrControl`, `Alt`, `Shift`). Capitalization mismatches cause parse failures.
- `src-tauri/capabilities/default.json -> windows` must list every allowed window label. Forgetting a new label causes "window not allowed" command errors.
- `include_bytes!` paths are relative to the source file, not the crate root.
- The ECDICT db must be declared in `tauri.conf.json -> bundle.resources` and must already exist on disk before `pnpm tauri build`. The file is gitignored and produced by `pnpm dict`; see the `Commands` section.
- The ECDICT `translation` column stores newlines as the literal two-character sequence `\n`. Unescape at the `dict.rs` boundary so every caller sees real newlines.
- rdev's Windows mouse hook runs on its own thread. Callbacks must not capture non-`Send` data.
- Window sizes in `tauri.conf.json` (`width`, `height`) are logical pixels (DIP). `WebviewWindow::set_position` takes physical pixels. When computing positions or clamping, query `WebviewWindow::outer_size()` for the actual physical size; do not assume the config values match.
- A window declared with `alwaysOnTop: true` and `focus: false` will still sink under the Windows taskbar after `show()` because the Z-order is decided by recency of activation. Re-assert `win.set_always_on_top(true)` after `win.show()` to force the topmost stack to refresh.
- With one HTML serving multiple windows, every webview is independent. Pinia store instances are not shared; cross-window communication goes through Tauri events.
- Do not install `@tauri-apps/cli` globally. Always use the project-local copy via `pnpm tauri` to avoid version drift.

## User-owned TODOs (do not auto-execute)

- Install Rust stable >= 1.85 (`winget install Rustlang.Rustup`, then `rustup default stable`).
- Install pnpm (`npm install -g pnpm`).
- Download the ECDICT csv to `scripts/data/ecdict.csv` (gitignored), then run `pnpm dict` to materialize `src-tauri/resources/ecdict.db` before the first `pnpm tauri build`.
