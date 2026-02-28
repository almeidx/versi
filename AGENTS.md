# AI Agent Instructions for Versi

## Project Overview

Versi is a native GUI application for managing Node.js versions.

The app is backend-agnostic and currently ships with:
- [fnm](https://github.com/Schniz/fnm) backend (`versi-fnm`)
- [nvm](https://github.com/nvm-sh/nvm) backend (`versi-nvm`)
- [Volta](https://github.com/volta-cli/volta) backend (`versi-volta`)

Adding a new backend requires implementing `BackendProvider` and `VersionManager` from `versi-backend` and wiring the provider into the app initialization path.

## Technology Stack

- **Language**: Rust (2024 edition)
- **GUI Framework**: [Iced](https://iced.rs/) 0.14 (Elm architecture)
- **Async Runtime**: Tokio
- **Build System**: Cargo workspace

## Project Structure

```
versi/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── versi/                    # Main GUI application
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       ├── app/              # Iced application handlers
│   │       │   ├── mod.rs        # Versi struct + app bootstrap
│   │       │   ├── init.rs       # Backend detection and initial load
│   │       │   ├── versions.rs   # Remote versions + metadata + schedule loading
│   │       │   ├── operations.rs # Install/uninstall/set-default/use operation handlers
│   │       │   ├── bulk_operations.rs # Bulk install/uninstall flows
│   │       │   ├── auto_update.rs # App update download/apply flow
│   │       │   ├── settings_io.rs # Import/export settings
│   │       │   ├── environment.rs # Native/WSL environment switching
│   │       │   ├── shell.rs      # Shell configuration handlers
│   │       │   ├── tray_handlers.rs # Tray event handlers
│   │       │   ├── window.rs     # Window state and geometry handling
│   │       │   ├── async_helpers.rs # Timeout/retry utilities
│   │       │   ├── update.rs     # Main message dispatch entry
│   │       │   ├── update/       # Dispatch split by concern
│   │       │   │   ├── navigation.rs
│   │       │   │   ├── operations.rs
│   │       │   │   ├── settings.rs
│   │       │   │   └── system.rs
│   │       │   └── platform/     # Platform-specific app integrations
│   │       │       ├── linux.rs
│   │       │       ├── macos.rs
│   │       │       ├── windows.rs
│   │       │       └── unsupported.rs
│   │       ├── message.rs        # Message enum (Elm-style)
│   │       ├── state/            # Application state modules
│   │       │   ├── mod.rs
│   │       │   ├── main.rs
│   │       │   ├── environment.rs
│   │       │   ├── onboarding.rs
│   │       │   ├── operations.rs
│   │       │   └── ui.rs
│   │       ├── theme/            # Theme palette and styles
│   │       ├── settings.rs       # User settings persistence
│   │       ├── logging.rs        # Debug log file management
│   │       ├── cache.rs          # Cached remote data persistence
│   │       ├── tray.rs           # System tray integration
│   │       ├── single_instance.rs # Single-instance enforcement
│   │       ├── views/            # UI views (main_view, settings_view, onboarding, loading, about)
│   │       └── widgets/          # Custom widgets (version_list, toast_container)
│   ├── versi-backend/            # Abstract backend traits and types
│   │   └── src/
│   │       ├── traits.rs         # BackendProvider, VersionManager, BackendDetection, BackendUpdate
│   │       ├── types.rs          # Shared types (NodeVersion, InstalledVersion, RemoteVersion, etc.)
│   │       ├── error.rs          # BackendError type
│   │       └── lib.rs            # Re-exports
│   ├── versi-core/               # Shared utilities (release schedule, app updates, metadata)
│   │   └── src/
│   │       ├── auto_update.rs    # Self-update download/extract/apply flow
│   │       ├── schedule.rs       # Node.js release schedule fetching
│   │       ├── metadata.rs       # nodejs.org index metadata fetching
│   │       ├── update.rs         # App update checking, GitHubRelease, version comparison
│   │       └── commands/mod.rs   # HideWindow trait + impls
│   ├── versi-fnm/                # fnm backend implementation
│   │   └── src/
│   │       ├── provider.rs       # FnmProvider - implements BackendProvider
│   │       ├── backend.rs        # FnmBackend - implements VersionManager
│   │       ├── version.rs        # Version parsing
│   │       ├── detection.rs      # fnm binary detection
│   │       ├── update.rs         # fnm update checking
│   │       └── error.rs          # Error types
│   ├── versi-nvm/                # nvm backend implementation
│   │   └── src/
│   │       ├── provider.rs       # NvmProvider - implements BackendProvider
│   │       ├── backend.rs        # NvmBackend - implements VersionManager
│   │       ├── client.rs         # nvm command execution abstraction
│   │       ├── detection.rs      # nvm detection
│   │       ├── update.rs         # nvm update checking
│   │       ├── version.rs        # nvm output parsing
│   │       └── error.rs          # Error types
│   ├── versi-volta/              # Volta backend implementation
│   │   └── src/
│   │       ├── provider.rs       # VoltaProvider - implements BackendProvider
│   │       ├── backend.rs        # VoltaBackend - implements VersionManager
│   │       ├── detection.rs      # volta detection
│   │       ├── update.rs         # volta update checking
│   │       └── version.rs        # volta output parsing
│   ├── versi-shell/              # Shell detection & configuration (backend-agnostic)
│   │   └── src/
│   │       ├── detect.rs         # Shell detection
│   │       ├── config.rs         # Config file editing (parameterized on marker/label)
│   │       └── verify.rs         # Configuration verification (parameterized on marker/backend_binary)
│   └── versi-platform/           # Platform abstractions
│       └── src/
│           ├── commands.rs       # HideWindow trait implementations
│           ├── paths.rs          # Platform-native paths
│           ├── environment.rs    # Environment abstraction
│           └── wsl.rs            # WSL distro detection (Windows)
```

## Architecture

### Elm Architecture (Model-View-Update)

The application follows Iced's Elm-style architecture:

1. **State** (`state/`): Immutable application state modules
2. **Message** (`message.rs`): Events that can modify state
3. **Update** (`app/update.rs` + dispatch modules): Handles messages and returns new state + tasks
4. **View** (`views/`): Pure functions that render state to UI

### Key Patterns

- **Tasks**: Async operations return `Task<Message>` for side effects
- **Subscriptions**: Time-based events (tick for toast timeouts)
- **Theming**: Dynamic light/dark themes based on system preference
- **Operation Queue**: Installs run concurrently (`active_installs: Vec<Operation>`), while uninstall and set-default are exclusive (`exclusive_op: Option<Operation>`). Pending operations are queued and drained when capacity is available.
- **System Tray**: Optional background tray icon with version switching support
- **Backend Abstraction**: The app resolves backend providers (`fnm`/`nvm`/`volta`) through `BackendProvider` and uses `VersionManager` trait objects. GUI/shell/platform crates avoid coupling to backend implementation details.

## Development Commands

```bash
# Build the project
cargo build --workspace

# Run the application
cargo run -p versi

# Run with release optimizations
cargo build --release --workspace

# Check for errors without building
cargo check --workspace

# Run tests
cargo test --workspace

# Format code
cargo fmt --all
cargo fmt --all -- --check

# Lint code (recommended strict local check)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Code Style Guidelines

- Follow standard Rust conventions (rustfmt)
- Use `thiserror` for error types
- Prefer typed error enums over stringly errors:
  - Do not introduce `Result<_, String>`, `Error(String)`, or `Box<dyn std::error::Error>` for internal/domain flows.
  - Avoid `map_err(|e| e.to_string())` in app/backend/shell/platform logic; preserve typed context and convert at UI/logging boundaries only.
  - When an error must be clone/equality-friendly for state/tests, use a structured detail enum (for example kind + message) instead of raw strings.
- Prefer `async/await` over callbacks
- Keep view functions pure (no side effects)
- Use meaningful message names that describe the event
- Group related functionality into separate crates
- Extract deterministic logic into small helpers and cover it with unit tests
- Keep tests local to modules when practical, and integration tests in `tests/` when cross-module behavior matters

## UI Guidelines

- **Toasts are only for background errors.** Never use toasts (`Toast::error`) for feedback that can be shown reactively in the UI (disabled buttons, inline text, tooltips, etc.). Toasts are reserved for errors from async background operations where no other UI surface exists to report the failure (e.g., install failed, uninstall failed, set-default failed).
- Prefer disabled states with tooltips, inline status text, or view changes over transient notifications.

## Key Files to Understand

1. `crates/versi/src/app/mod.rs` - Main application logic and message dispatch
2. `crates/versi/src/app/update.rs` - Main update dispatcher and routing handoff
3. `crates/versi/src/state/main.rs` - Main app state and environment coordination
4. `crates/versi/src/message.rs` - All possible application events
5. `crates/versi-backend/src/traits.rs` - `BackendProvider` and `VersionManager` trait definitions
6. `crates/versi-fnm/src/provider.rs` - `FnmProvider` (concrete backend implementation)
7. `crates/versi-nvm/src/provider.rs` - `NvmProvider` (concrete backend implementation)
8. `crates/versi-volta/src/provider.rs` - `VoltaProvider` (concrete backend implementation)

## Common Tasks

### Adding a New Feature

1. Add new message variant(s) to `message.rs`
2. Add state fields in `state/` modules if needed
3. Route and handle message in `app/update/*` dispatch handlers
4. Update view in appropriate `views/` file
5. Add unit tests for any deterministic logic introduced

### Adding a New Backend

1. Create a new crate (e.g., `versi-volta`), following `versi-fnm` as a reference
2. Implement `BackendProvider` trait (detection, installation, update checking)
3. Implement `VersionManager` trait (list installed/remote, install, uninstall, set default)
4. Add the backend kind to `crates/versi/src/backend_kind.rs`
5. Wire the provider into initialization in `crates/versi/src/app/mod.rs` / `crates/versi/src/app/init.rs`

### Modifying Styles

- Theme root: `crates/versi/src/theme/mod.rs`
- Style functions: `crates/versi/src/theme/styles/`
- Reuse existing style helpers instead of introducing ad-hoc inline style closures when possible

## Testing

- Unit tests should be in the same file as the code
- Integration tests should live in `tests/` directories (example: `crates/versi-shell/tests/`)
- Prefer deterministic tests that avoid external network/process dependencies unless specifically testing those integrations
- Before finishing work, run:
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Dependencies

Key external crates:
- `iced` - GUI framework
- `tokio` - Async runtime
- `reqwest` - HTTP client (for release schedule)
- `serde` - Serialization
- `open` - Opening URLs in browser
- `dirs` - Platform directories
- `which` - Finding executables
- `zbus` - Linux launcher badge integration
- `gtk` - Linux display integration/bootstrap

## Data & Storage

**Settings Location:**
- macOS: `~/Library/Application Support/versi/`
- Windows: `%APPDATA%/versi/`
- Linux: `~/.config/versi/` (XDG-compliant)

**Cached Data:**
- Available Node versions list (fetched from nodejs.org)
- Node.js release schedule (from GitHub)

## Backend Interaction

The GUI interacts with backends exclusively through the `BackendProvider` and `VersionManager` traits defined in `versi-backend`.

- All operations run as async tasks, keeping the UI responsive via Iced's `Task` system
- Parse stdout/stderr for status and results
- Multiple installs can run concurrently; uninstall and set-default wait for all installs to finish

**Key fnm commands used (in `versi-fnm`):**
- `fnm list` - Get installed versions
- `fnm list-remote` - Get available versions
- `fnm install <version>` - Install a version
- `fnm uninstall <version>` - Remove a version
- `fnm default <version>` - Set default version
- `fnm current` - Get currently active version

**Key nvm commands used (in `versi-nvm`):**
- `nvm ls` - Get installed versions
- `nvm ls-remote` - Get available versions
- `nvm install <version>` - Install a version
- `nvm uninstall <version>` - Remove a version
- `nvm alias default <version>` - Set default version
- `nvm current` - Get currently active version

**Key Volta commands used (in `versi-volta`):**
- `volta list node --format plain` - Get installed Node runtimes
- `volta fetch node@<version>` - Fetch a Node runtime
- `volta install node@<version>` - Set global default Node runtime
- `volta list --current node --format plain` - Get active Node runtime
- `volta list --default node --format plain` - Get default Node runtime

## Platform-Specific Notes

### macOS
- Primary development target
- Native ARM64 and x64 binaries
- Uses `dark-light` crate for system theme detection

### Windows
- Native Windows binary
- Support for PowerShell shell configuration
- WSL integration via `wsl.exe` for multi-environment support

### WSL (Windows Subsystem for Linux)
- Accessed via Windows app's multi-environment support
- Lists all WSL distros via `wsl.exe --list --verbose`
- Separately checks which distros are running via `wsl.exe --list --running --quiet`
- Only checks for the backend in running distros (avoids booting non-running distros)
- Detects backend paths using backend-specific search paths (e.g., `~/.local/share/fnm/fnm`, `~/.nvm/nvm.sh`, `~/.volta/bin/volta`)
- Shows all distros as tabs; non-running or backend-less distros appear disabled with reason
- Commands executed directly via `wsl.exe -d <distro> /path/to/backend ...` (no shell needed)
- Shell detection in settings is environment-aware: shows Linux shells (bash/zsh/fish) for WSL environments

### Linux
- Native x64 and ARM64 binaries
- XDG-compliant paths
- Support for bash, zsh, fish shells
