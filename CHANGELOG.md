# Changelog

All notable changes to this project will be documented in this file.


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

