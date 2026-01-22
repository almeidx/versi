# Changelog

All notable changes to this project will be documented in this file.


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

