# Changelog

All notable changes to this project will be documented in this file.


## [0.12.1] - 2026-03-06

- refactor: replace fd-lock with std File::try_lock
- refactor: restrict version parse functions to pub(crate) visibility
- refactor: remove redundant test_ prefix from test function names
- refactor: avoid unnecessary clone in NVM provider detect
- refactor: add missing Debug derive to FnmBackend and AsdfBackend
- refactor: restrict NVM internal types and functions to pub(crate) visibility
- refactor: restrict update check functions to pub(crate) visibility
- refactor: make commands module private in versi-core
- refactor: remove unused public re-exports
- refactor: remove duplicated format tests from version list item
- refactor: remove unused shells re-export module
- perf: avoid intermediate allocation in response_snippet
- refactor: extract shared combine_error_output helper
- refactor: remove unused public re-exports from backend crates
- refactor: reduce public API surface of internal symbols
- refactor: remove unused methods and dead code
- refactor: extract shared run_unix_install_script helper
- refactor: remove redundant data_dir clone and with_asdf_data_dir call
- perf: avoid unnecessary String allocations in update check and banner formatting
- refactor: derive Copy for NvmVariant and ApplyResult
- refactor: remove duplicate shell_type_to_str in favor of ShellType::shell_arg
- refactor: derive Copy for ShellType and pass by value
- refactor: extract shared write_mock_executable! macro to versi-backend
- perf: use split_once and avoid intermediate allocations in shell config
- chore: update Cargo.lock after sha2 removal from versi-asdf
- refactor: reduce public API surface of internal symbols
- refactor: use &Path instead of &PathBuf in function parameters
- refactor: remove unused ShellConfigEdit.original field and diff_preview method
- refactor: remove unused download_install_script_verified and related dead code
- refactor: add #[must_use] to ShellConfig::add_init and update_flags
- refactor: use &Path instead of &PathBuf in nvm detection
- test: remove duplicate is_newer_version tests from versi-nvm
- chore: remove unused sha2 dependency from versi-asdf
- perf: derive Copy for NodeVersion and remove redundant clones
- refactor: extract download_and_prepare_install_script to versi-backend
- fix: add missing User-Agent headers to 4 HTTP request functions
- refactor: standardize logging levels across all backends
- refactor: remove AppError::Message variant and serde_json string escape hatch
- perf: optimize toast IDs, fetch clones, and environment load dedup
- refactor: standardize User-Agent to Versi/{version} across all HTTP clients
- refactor: deduplicate version prefix stripping and fix sentinel check
- refactor: remove unused install_date field from InstalledVersion
- refactor: replace fs2 with fd-lock for file locking
- refactor: remove unused execute_in_wsl and WslError from versi-platform
- perf: use full LTO for smaller release binaries
- test: add diagnostic output to fnm mock command failure test
- test: add compile-time safety tests for BackendKind/provider name mapping
- test: add tests for auto_update state machine
- test: add tests for shell configuration handlers
- test: add missing provider tests for versi-nvm
- refactor: make FnmBackend::execute delegate to shared execute_backend_command
- fix: use Result::ok method reference in fs_utils tests
- refactor: extract duplicate `replace_file` and quarantine logic to fs_utils module
- perf: remove intermediate Vec collect in background preload scheduling
- perf: add fast path for `sanitize_terminal_text` when input has no escape sequences
- refactor: remove dead `fnm_dir` and `node_dist_mirror` settings fields
- fix: remove unnecessary `providers.clone()` in Versi::new
- refactor: merge duplicate `should_confirm_default_uninstall` / `is_already_default_version`
- fix: widen timeout test gap for Windows timer granularity
- refactor: deduplicate provider test suites with shared macro
- refactor: extract shared CommandEnvironment and command building to versi-backend
- chore: update Cargo.lock after dependency changes
- refactor: split auto_update.rs into download, extract, and apply phases
- refactor: split state/operations.rs into modal, bulk, and operations
- refactor: extract find_default_version helper to versi-backend
- refactor: remove redundant internal detection structs
- refactor: replace crossbeam-channel with MenuEvent::set_event_handler
- refactor: extract InstallerAttempt to versi-core
- refactor: extract download_install_script to versi-core
- refactor: extract parse_current_version helper to versi-backend
- refactor: extract check_github_backend_update helper to versi-core
- test: add unit tests for get_cli_version prefix-stripping
- refactor: remove duplicate asdf_data_dir field in AsdfBackend
- chore: run formatter
- fix: add nvm prefix to normalize_backend_version_output
- fix: clear fetch errors consistently on retry
- refactor: unify clamp helpers and use default functions in AppSettings
- fix: use cfg(unix) instead of cfg(not(windows)) in settings
- refactor: derive Default for FetchState and VersionCache
- test: add unit tests for format_relative_time
- refactor: deduplicate fetch error banner functions
- refactor: precompute AppPaths in state instead of in view
- refactor: extract items_matching helper in BulkRunState
- perf: precompute LTS codename lookup map instead of O(n) scan
- refactor: extract shared init task builder
- test: add unit tests for is_major_release
- refactor: extract shared format_tenths to dedicated module
- fix: make command_output_to_result tests cross-platform
- chore: run formatter
- refactor: replace into_backend_update with From impl
- refactor: extract shared command execution helper to versi-backend
- perf: reuse shared reqwest::Client instead of creating new instances
- refactor: deduplicate CLI version probe across backends
- test: add unit tests for auto_update helpers
- test: add unit tests for version list filtering and sorting
- refactor: extract shared temp_script_path to versi-core
- refactor: preserve typed context in AppErrorDetail core error conversions
- refactor: extract shared GitHub release check to versi-core
- refactor: remove redundant fnm_dir field from FnmBackend
- refactor: use into_owned() instead of to_string() on Cow<str>
- fix: use typed errors in asdf Windows installer
- fix: scope detect_options to marker lines only
- deps: Update patch/minor dependencies (#84)
- refactor: deduplicate ANSI stripping across backends
- fix: log debug warnings for silent version detection failures
- fix: use typed errors in NVM Windows installer
- perf: reuse shared reqwest::Client in Volta backend
- fix(windows): quote paths in WSL shell command interpolation
- fix: log warnings instead of silently swallowing cache save errors
- refactor: extract duplicated backend update checking to versi-core
- refactor: extract duplicated `response_snippet` to shared module
- perf: improve startup performance
- fix(windows): convert WSL detection to async with configurable timeouts
- deps: Lock file maintenance (#83)


## [0.12.0] - 2026-02-28

- fix(windows): default wgpu to low power preference to prevent startup hang
- chore: run formatter
- chore: remove unwanted changes from changelog
- feat(onboarding): add unsafe backend auto-install flows
- Align backend behavior and tests across managers
- feat: add asdf backend (nodejs plugin)
- feat: add volta backend support
- deps: Update Rust crate objc2 to 0.6.4 (#81)
- deps: Update GitHub Artifact Actions (#80)
- fix: warning on non-windows
- Redesign and center About page content
- Move bulk progress banner to bottom overlay


## [0.11.0] - 2026-02-25

- feat: add overlay scrollbars and sticky section headers
- fix: trap scroll on modals
- Integrate Node security advisory warnings and caching
- deps: Update patch/minor dependencies (#78)
- Fix fnm mock command test
- Fix keyboard list-mode version actions and search input bleed
- Implement background env preload
- Add bulk progress and preload envs
- Add bulk operation progress UI
- test(nvm): retry uninstall when target version is active
- test(ci): remove WSL integration tests and workflow job
- Fix tray tooltip update
- test: add backend integration tests and CI workflow
- Add tray tooltip version
- Add detailed install progress
- deps: Update Rust crate zbus to 5.14.0 (#77)
- deps: Lock file maintenance (#76)


## [0.10.3] - 2026-02-20

- fix(windows): align launch-at-login registry status type
- fix(clippy): gate home-dir launch error to macOS
- fix(clippy): pass Linux badge worker receiver by reference
- refactor(app): use direct error mappers for environment loads
- docs(agents): codify typed error handling requirements
- refactor(cache): type app-path resolution load errors
- refactor(tray): remove type-erased tray and badge errors
- refactor(app-errors): introduce typed AppError details and mappings
- refactor(platform): type launch-at-login integration errors
- refactor(shell): replace verification string errors with typed enums
- refactor(single-instance): replace string lock errors with typed variants
- refactor(errors): type backend parse/network/install failures
- refactor(update): use GitHub asset digest for verification
- perf(linux): reuse single worker for badge updates
- fix(cache): add structured load errors and corrupt-file quarantine
- fix(network): log configured HTTP client build fallback
- fix(install): verify backend installer scripts with retries
- perf(cache): coalesce incremental saves on bounded worker queue
- perf(settings): debounce non-critical settings writes


## [0.10.2] - 2026-02-20

- build(deps): drop reqwest decompression codec features
- build(ui): disable iced defaults with explicit runtime features
- build(deps): minimize reqwest chrono and simplelog features
- fix(windows): restore compile on CI and clean platform warnings
- build(deps): disable tokio defaults and trim tokio-util
- build(deps): reduce default dependency feature surface
- refactor(tests): use main state helpers in app tests
- chore(windows): document Win32 unsafe invariants
- perf(queue): use hash set for install dedup in drain
- perf(version): cut string allocations in hot compare paths
- fix(nvm): avoid path interpolation in shell bootstrap
- fix(tray): route set-default by stable environment id
- fix(storage): use atomic replace semantics on Windows
- fix(tray): make subscription worker shutdown deterministic
- fix(windows): locate Versi window without exact title match
- fix: clippy warning
- perf(search): reduce per-keystroke allocation churn
- refactor(state): simplify latest-by-major recomputation
- refactor(shell): type WSL shell configuration errors
- refactor(platform): type app path resolution errors
- chore: fix formatting
- refactor(auto-update): replace string errors with typed variants
- refactor(core): use typed errors for remote fetch operations
- refactor(tray): replace polling loop with blocking event bridge
- fix(settings): sanitize persisted values and write atomically
- fix(instance): enforce single-instance lock on non-windows
- refactor(update): use semver for app version comparisons
- fix(network): validate HTTP status for metadata and schedule fetches
- fix(wsl): parse distro names with embedded spaces
- fix(platform): quote launch-at-login executable paths
- refactor(security): replace piped installer scripts with downloaded temp scripts
- feat(security): verify update artifacts with published sha256 checksums
- feat(settings): add configurable app update behavior
- fix(wsl): use sh instead of bash in execute_in_wsl for portability
- refactor: preserve io::ErrorKind in BackendError::IoError conversion
- refactor: extract tick rate magic numbers into named constants
- refactor: derive filter chips from a const table instead of a fixed-size array
- refactor: remove trivial NodeVersion::major_group() getter
- refactor: remove redundant `..` in Operation::Install match arm
- refactor: derive VersionParseError Display/Error via thiserror
- refactor: return static slices from wsl_search_paths instead of Vec


## [0.10.1] - 2026-02-19

- refactor: update VersionGroup::from_versions to accept slices instead of vectors
- refactor(platform): single-pass fold for WSL distro summary counts
- refactor(nvm): move ANSI stripping from version parsing to client layer
- fix(shell): return empty config paths when home directory is unavailable
- Replace unsafe byte-offset string slicing in nvm version parsing
- Extract main_state/onboarding_state test helpers to reduce boilerplate
- Use split_once and strip_prefix/suffix in fnm version parsing
- Reuse versi_core::GitHubRelease in nvm update module
- Reduce unnecessary cloning in NvmProvider
- fix(nvm,platform): log debug messages for silent parse failures
- refactor(platform): deduplicate WslDistro construction in parse_wsl_list
- fix(core): sanitize download filename extracted from URL
- refactor(nvm): remove duplicated is_newer_version, use versi-core's
- refactor(views): simplify app_update_badge with badge_btn and external_link_btn helpers
- refactor(theme): extract hardcoded layout dimensions into token constants
- refactor(theme): centralize repeated color literals into token constants
- rename(theme): tahoe → tokens
- refactor(theme): add RADIUS_XS constant and replace hardcoded border radii
- refactor(backend): remove redundant FnmError and NvmError enums
- refactor(backend): replace Box<dyn VersionManager> with Arc
- refactor(nvm): remove Mutex anti-pattern from NvmProvider
- fix(backend): return BackendError from check_for_update instead of String
- fix(tray): log warning for unknown menu event IDs
- refactor(views): extract duplicated has_tabs padding into content_padding
- deps: Update Rust crate zip to 8.1.0 (#71)
- chore: fix formatting
- chore(fnm): remove dead _check_fnm_update function and serde_json dep
- refactor(bulk_ops): deduplicate latest_by_major helpers
- refactor(state): unify Operation and OperationRequest enums
- refactor(state): extract FetchState to reduce VersionCache complexity
- chore: remove unused dependencies
- fix(nvm): log suppressed error in default_version()
- fix(shell): use boundary-aware matching for flag removal
- deps: Update patch/minor dependencies (#70)


## [0.10.0] - 2026-02-17

- refactor(versions): split fetch and update-check handlers
- refactor(versions): move handler tests into dedicated module file
- refactor(versions): extract cache-save worker module
- perf(search): cache normalized remote version query fields
- refactor(search): centralize shared query and release filter logic
- test(perf): add ignored search and grouping baseline checks
- refactor(banners): precompute update and eol counts in state
- test(metadata): add navigation and banner regression coverage
- refactor(errors): complete runtime migration to typed app errors
- refactor(update): centralize URL open task helper
- refactor(errors): use typed app-update error variants
- refactor(errors): use typed operation failure errors
- build(release): enable thin-lto and binary stripping
- test(filters): add alias and limit-order regression coverage
- refactor(available-row): remove unreachable action branch
- feat(metadata): track metadata fetch errors and show retry banner
- refactor(version-rows): reduce eager string cloning in row actions
- refactor(versions): debounce background cache persistence
- refactor(cache): write disk cache via atomic temp-file replace
- fix(search): apply filters before limits and to alias results
- refactor(errors): add structured app errors for settings and fetch flows
- refactor(version-list): extract group/query predicates with tests
- refactor(async): cancel stale environment and version fetch tasks
- refactor(state): use typed installed version sets
- refactor(search): unify query engine across state and widgets
- refactor(widgets): extract nav state helpers with tests
- refactor(version-list): extract row action and badge logic
- refactor(search): extract chip state helpers and add tests
- docs(crates): expand public crate-level API docs
- refactor(operations): extract queue and toast helpers
- refactor(tray): split tray event handlers into helpers
- refactor(errors): use structured shell and backend variants
- test(state): cover network status and navigation queries
- test(update): cover context-menu dismissal rules
- test(tray-handlers): cover tray event state transitions
- test(tray): cover menu shaping and event parsing
- test(shell): verify status mapping and backend kind
- test(auto-update): cover update-state transitions
- test(onboarding): cover state transitions and shell mapping
- test(init): cover backend selection helpers
- test(app): cover window lifecycle state transitions
- fix(widget): honor version filter limit and add coverage
- test(app): cover remote, schedule, and metadata update handlers
- chore: fix formatting
- test(app): cover environment filters and operation queueing
- test(app): add dispatch coverage for update routing modules
- test(linux): avoid identical match arms in wayland fallback test
- fix(linux): satisfy wayland env lookup fn lifetime bounds
- docs: refresh README and AI guide for current architecture
- style: format rust sources after test expansion
- test: address strict clippy lints in new tests
- test(app): cover bulk operation version selection logic
- test(app): add coverage for theme and single-instance helpers
- test(core): cover auto-update zip extraction helpers
- test(shell): cover verify path selection and wsl guards
- test(app): add cache and logging helper coverage
- test(backends): add error and provider coverage for nvm/backend
- test(ui): cover version size formatting helpers
- test(app): add coverage for async helpers and wayland detection
- test(state): cover constructors and environment state updates
- test(app): add unit coverage for settings and error types
- test(core): add metadata mapping unit tests
- test(fnm): add coverage for provider, detection, and update logic
- test(platform): cover environment and path helpers
- test(backend): cover version manager trait defaults
- chore: fix formatting
- fix: satisfy strict clippy warnings on linux platform
- refactor: store window geometry positions as floats
- docs: add errors sections for public result APIs
- refactor: split large settings and version list views
- refactor: split init and update dispatch handlers
- chore: enable workspace pedantic clippy by default
- refactor: clean pedantic warnings in views and widgets
- refactor: tighten pedantic compliance in app core
- chore: clean pedantic warnings in backend crates
- fix(version-list): remove label-only hover affordance
- fix(version-list): make full row left-clickable
- test(operations): add generated invariant checks for queue draining
- test: cover settings file IO and shell config edit flows
- chore(format): normalize windows overlay icon call formatting
- perf(cache): reduce cloning in latest-by-major recomputation
- refactor(windows): remove unwraps in overlay icon path
- refactor(platform): split platform helpers into per-OS modules
- refactor(app): extract shared async timeout and retry helpers
- fix(async): guard environment and version fetches by request sequence
- refactor(state): store AppError in UI state
- refactor(app): propagate AppError across remaining message flows
- refactor(message): box heavy update payload variants
- refactor(app): introduce typed AppError for core operation flows
- test(app): add update dispatcher routing coverage
- refactor(settings): deduplicate settings save and shell option updates
- chore(clippy): box forwarded messages in update dispatch
- test(environment): cover load failure and recovery paths
- refactor(app): split update dispatcher by domain
- chore(clippy): satisfy strict warnings and format touched code
- refactor(app): move message dispatcher out of app/mod.rs
- fix(environment): surface version load failures instead of swallowing
- test(app): add cross-environment tray/backend regression coverage
- refactor(app): replace raw backend strings with typed BackendKind
- refactor(app): extract settings IO handlers and add dispatcher tests
- fix(nvm): execute commands with argv instead of string parsing
- fix(init): remove leaked fallback string in WSL backend detection
- fix(shell): write WSL shell config changes to target distro
- fix(app): keep active provider aligned with selected environment
- refactor: reduce parameter threading and deduplicate styles
- feat: add right-click context menu on version rows
- deps: Update Rust crate zip to v8 (#67)


## [0.9.0] - 2026-02-15

- feat: add version detail modal with inline release metadata
- feat: add launch at login setting with per-platform autostart
- feat: add search filter chips for LTS, installed, not installed, EOL, and active
- fix: remove unnecessary libxdo dependency
- deps: Update patch/minor dependencies (#64)
- fix(windows): update Windows API calls for newer windows crate and add GDI RAII cleanup
- feat: add update badge on app icon and adopt reverse-domain app ID
- fix: update icon and desktop caches in Linux install script
- deps: Lock file maintenance (#63)


## [0.8.5] - 2026-02-07

- fix: set application_id on Linux for GNOME/Wayland icon matching
- deps: Update Rust crate reqwest to 0.13.2 (#61)
- feat: add confirmation modal before uninstalling default version
- docs: add module-level documentation to app handler files
- refactor: remove duplicate HideWindow trait implementations
- fix: replace expect() panics with graceful error handling in AppPaths
- fix: log settings save failures instead of silently ignoring
- fix: handle deleted inode path in Linux restart after self-replace


## [0.8.4] - 2026-02-07

- fix: only use minimize fallback on Wayland, use Mode::Hidden on X11
- fix: use pkexec for Linux self-update when binary is in system path
- deps: Update Rust crate zip to 7.4.0 (#59)


## [0.8.3] - 2026-02-06

- deps: Update Rust crate zip to 7.3.0 (#57)
- fix: prevent MSI temp file deletion during Windows auto-update
- feat: toggle tray menu between Open/Hide Versi based on window state
- feat: add Linux support to install.sh
- fix: suppress unused variable warning in Windows apply_update
- fix: use minimize instead of Mode::Hidden on Linux for tray hide


## [0.8.2] - 2026-02-05

- chore(deps): bump time in the cargo group across 1 directory (#55)
- fix: process GTK events for tray icon on Linux (Bazzite)


## [0.8.1] - 2026-02-04

- fix: hide unloaded/unavailable environments from system tray menu
- fix: reload shell setup list when switching environments in settings
- feat: add Ctrl+Tab / Ctrl+Shift+Tab for environment switching
- feat: show "Updating..." on bulk update banner while operations run
- fix: correct Windows explorer /select arg for "Show in Folder"


## [0.8.0] - 2026-02-04

- ci: remove security audit job
- deps: update bytes 1.11.0 -> 1.11.1 (RUSTSEC-2026-0007)
- feat: add "Show in Folder" button for settings config file
- feat: make hardcoded values configurable via settings file
- refactor: make shell options per-backend instead of global
- feat: add version alias resolution (latest, lts/*, lts/<codename>)
- perf: cache latest_by_major and installed_set to avoid per-frame recomputation
- feat: add keyboard shortcuts help modal (? key)
- feat: show cache age in offline mode banner
- feat: add settings export/import
- fix: handle missing system tray on Linux (Bazzite)
- deps: Update Rust crate tempfile to 3.24.0 (#51)
- fix: prevent tray interaction from bricking app during loading/onboarding


## [0.7.0] - 2026-02-02

- feat: add in-app auto-update with download, extract, and apply
- ci: use rustsec/audit-check action for security scanning
- refactor: replace dark-light with Iced native theme detection
- ci: add cargo-audit ignore list for non-actionable advisories
- ci: add cargo-audit security scanning
- refactor: remove install progress tracking


## [0.6.1] - 2026-02-02

- feat: add keyboard navigation for version list
- test: add unit tests for versi-nvm backend
- test: add comprehensive unit tests for OperationQueue
- revert: use std::thread::spawn for cache writes
- refactor: split app/mod.rs by extracting window and bulk operation handlers
- refactor: extract OperationQueue methods and split bulk operations
- fix: add timeouts to all backend operations to prevent UI freezes
- fix: use tokio::task::spawn_blocking for cache writes instead of std::thread::spawn
- fix: validate saved window geometry before restoring position
- fix: recover from poisoned Mutex in NvmProvider instead of panicking
- fix: ignore tray events before app finishes initializing
- fix(win): keep nav buttons stable when switching views with environment tabs
- deps: Lock file maintenance (#47)
- chore: update screenshot


## [0.6.0] - 2026-01-31

- refactor: extract fnm backend from versi-core into versi-fnm
- fix: resolve Windows build error and unused import warnings
- fix(win): build
- refactor: unify navigation header across all views
- refactor: eliminate all #[allow] attributes via structural changes
- refactor: remove dead code and unused enum variants/fields
- refactor: centralize tooltip styling with styled_tooltip helper
- refactor: normalize spacing and header size in Settings and About views
- fix: improve header visibility and declutter search results
- refactor: compact header and normalize spacing in main view
- docs: add UI guideline restricting toast usage to background errors
- feat: add nvm backend with per-environment engine selection


## [0.5.0] - 2026-01-30

- fix: resolve Windows build errors in shell setup handler
- feat: animate refresh icon with spinning rotation during environment load
- feat: add keyboard shortcuts for search focus, settings, and refresh
- refactor: make codebase engine-agnostic with BackendProvider trait
- refactor: rename FnmUi to Versi and split large files into modules
- refactor: remove unused function parameters and redundant data
- feat: replace toasts and overlays with inline reactive UI feedback
- fix: fnm detection and window close on Linux/GNOME
- feat: clean up config, cache, and log data on uninstall
- feat: position scrollbars flush against the right window edge
- feat: remove install success toasts in favor of inline UI feedback
- feat: streamline onboarding by removing Install Node and Complete steps


## [0.4.0] - 2026-01-30

- deps: Update Rust crate gtk to 0.18.2 (#43)
- feat: add network error handling, disk caching, and retry with backoff
- feat: replace Unicode icons with embedded SVG icons
- feat: persist window size and position across sessions
- feat: add About as a separate tab accessible from homepage and tray


## [0.3.3] - 2026-01-29

- chore: lockfile
- fix: truncate debug log file when it exceeds 5MB
- fix: remove default version from header subtitle
- feat: add arrow icon to browser-opening buttons
- fix: recover from poisoned mutex in logging
- feat: add tooltips to icon buttons
- fix: replace .unwrap() calls with safe patterns in app.rs
- fix: initialize GTK before tray icon on Linux
- chore: release v0.3.2 (#41)
- fix: use detected fnm path for initial environment load
- fix: improve badges and update screenshot


## [0.3.2] - 2026-01-28

- fix: use detected fnm path for initial environment load
- fix: improve badges and update screenshot


## [0.3.1] - 2026-01-28

- fix: populate tray menu when starting minimized


## [0.3.0] - 2026-01-28

- chore: relicense to GPL-3.0-only
- fix: correct UTF-16LE detection in WSL output decoder
- docs: update CLAUDE.md with current project structure and features
- feat: add search clear button, hide installed during search, changelog link icon
- fix: limit visible toasts to 3 most recent
- fix: reduce top padding when environment tabs are absent
- fix: make modal background opaque and suppress hover bleed-through
- feat: support parallel install operations
- feat: add Open Versi and Settings items to system tray menu
- feat: overhaul UI with settings page, Tahoe aesthetics, and contextual actions
- test: add comprehensive unit tests for pure functions
- feat: add macOS install script


## [0.2.1] - 2026-01-28

- fix: check for fnm updates when switching environments
- feat: warn user when uninstalling default version
- fix: reduce debug log noise
- fix: keep showing versions during refresh instead of loading screen
- fix(windows): verify shell config inside WSL instead of on Windows host
- fix(windows): improve PowerShell detection and add WSL shell logging
- fix(macos): delay minimize until after first environment loads
- fix(macos): auto-refresh versions when showing window from tray


## [0.2.0] - 2026-01-28

- fix(ci): use cargo update --workspace to avoid updating external deps
- docs: update documentation for WSL and bulk operations
- feat: add "Keep Latest" button to uninstall all versions except latest in major
- fix(windows): show Linux shells in settings when WSL environment is active
- fix(macos): ensure versions load when starting minimized to tray
- fix(windows): allow window to show after starting minimized to tray
- fix(windows): show unavailable WSL distros as disabled instead of hiding them
- deps: Update patch/minor dependencies (#35)


## [0.1.2-alpha.20] - 2026-01-26

- fix: hide to tray instead of exiting when close button clicked
- fix: hide window instead of minimize when tray is always running


## [0.1.2-alpha.19] - 2026-01-26

- fix(ci): checkout merge commit for release tag
- ci: improve Rust cache sharing between workflows
- fix: bulk update only compares latest installed version per major
- chore: upgrade to Rust 2024 edition
- chore: release v0.1.2-alpha.18 (#32)
- fix(windows): add Win32_Security feature for CreateMutexA
- refactor: consolidate install into main search
- chore: remove dead code and unnecessary allow attributes
- feat: add operations queue, bulk operations, and Windows fixes
- deps: Lock file maintenance (#31)
- deps: Update Rust crate winresource to 0.1.30 (#30)
- deps: Update patch/minor dependencies (#29)


## [0.1.2-alpha.18] - 2026-01-26

- fix(windows): add Win32_Security feature for CreateMutexA
- refactor: consolidate install into main search
- chore: remove dead code and unnecessary allow attributes
- feat: add operations queue, bulk operations, and Windows fixes
- deps: Lock file maintenance (#31)
- deps: Update Rust crate winresource to 0.1.30 (#30)
- deps: Update patch/minor dependencies (#29)


## [0.1.2-alpha.17] - 2026-01-23

- feat: add system tray with quick-switch menu
- fix: show correct fnm version per environment


## [0.1.2-alpha.16] - 2026-01-23

- chore: fix clippy warning and apply cargo fmt
- feat: enable/disable debug logging without restart
- feat: add log file stats, clear button, and reveal in folder
- fix: recreate log file if deleted while app is running
- fix: add right padding to settings modal for scrollbar
- feat: click to copy debug log path to clipboard
- fix(wsl): return only first found fnm path instead of all matches


## [0.1.2-alpha.15] - 2026-01-23

- feat: add debug logging with settings toggle
- docs: update WSL documentation to reflect new implementation
- refactor(wsl): detect fnm binary path directly instead of using shell
- deps: Update Rust crate winresource to 0.1.29 (#25)


## [0.1.2-alpha.14] - 2026-01-22

- refactor(wsl): detect and cache user's default shell
- fix(wsl): use user's default shell instead of hardcoding bash


## [0.1.2-alpha.13] - 2026-01-22

- fix(wsl): capture and display actual error messages for install failures


## [0.1.2-alpha.12] - 2026-01-22

- fix(wsl): explicitly source shell config files before running fnm
- fix(installer): convert semantic version to MSI-compatible format


## [0.1.2-alpha.11] - 2026-01-22

- fix(wsl): only detect running WSL distros to avoid starting WSL
- fix(wsl): run fnm commands through login shell and improve settings UX


## [0.1.2-alpha.10] - 2026-01-22

- fix(win): wsl detection
- chore: update icons


## [0.1.2-alpha.9] - 2026-01-22

- fix(win): imports
- Release v0.1.2-alpha.8 (#18)
- fix(win): imports
- chore: release v0.1.2-alpha.7 (#17)
- fix(windows): add window icon to title bar
- feat: add about section
- feat: add WSL environment tabs for Windows
- refactor: restructure release workflow for immutable releases


## [0.1.2-alpha.8] - 2026-01-22

- fix(windows): add window icon to title bar
- feat: add about section
- feat: add WSL environment tabs for Windows
- refactor: restructure release workflow for immutable releases


## [0.1.2-alpha.7] - 2026-01-22

- fix(windows): add window icon to title bar
- feat: add about section
- feat: add WSL environment tabs for Windows
- refactor: restructure release workflow for immutable releases


## [0.1.2-alpha.6] - 2026-01-22

- fix(wix): move Icon element to Package level
- fix: use cargo generate-lockfile instead of cargo check
- fix: misc release and UI improvements
- feat: add app icon for all platforms
- fix(win): hide console windows when spawning subprocesses
- fix: sync detected shell options to settings toggles
- fix(win): license
- fix: misc improvements


## [0.1.2-alpha.5] - 2026-01-22

- fix(win): run as gui
- feat: add changelog button to homepage
- feat: add EOL badges and allow installing non-LTS versions
- fix: make operation status and toasts float over content
- fix: container background
- fix: improve shell configuration UI and toggle behavior
- deps: Update patch/minor dependencies (#13)


## [0.1.2-alpha.4] - 2026-01-21

- chore: add version to release asset filenames
- refactor: rebrand from fnm-ui to Versi and add backend abstraction
- feat: add configurable shell init options
- fix: resolve clippy warning in detect_fnm_dir


## [Unreleased]

- chore: rebrand from fnm-ui to Versi
  - Renamed all crates: fnm-ui → versi, fnm-core → versi-core, fnm-shell → versi-shell, fnm-platform → versi-platform
  - Updated window titles, theme names, and onboarding text
  - Updated settings directory from fnm-ui to versi
  - Updated GitHub repository references to almeidx/versi
  - Updated all release artifacts and installers

## [0.1.2-alpha.3] - 2026-01-21

- fix: auto-detect FNM_DIR for GUI app bundles


## [0.1.2-alpha.2] - 2026-01-21

- feat: add Windows MSI installer
- feat: create proper app bundles for all platforms
- fix: Don't bump version when updating prerelease identifier


## [0.1.2-alpha.1] - 2026-01-21

- ci: Use ARM runner for Linux ARM64 builds

## [0.1.1-alpha.0] - 2026-01-21

- chore: Reset version for re-release
- ci: Optimize release builds to use fewer runners
- chore: prepare release v0.1.1-alpha.0 (#7)
- fix: Force push release branch to handle retries
- fix: Fix YAML syntax in release-prepare workflow
- deps: Update actions/download-artifact action to v7 (#6)
- ci: Redesign release workflow to use PR-based approach
- deps: Update patch/minor dependencies (#5)
- deps: Update Rust crate which to v8 (#3)
- deps: Update GitHub Artifact Actions (#2)
- deps: Update actions/checkout action to v6 (#1)
- chore: cargo fmt
- chore: add renovate config
- fix: resolve all clippy warnings
- feat: add app update checking
- fix: resolve clippy warnings
- style: apply cargo fmt formatting
- ci: add concurrency to cancel duplicate runs
- fix(ci): use correct rust-toolchain action name
- Initial commit: fnm-ui - GUI for Fast Node Manager
