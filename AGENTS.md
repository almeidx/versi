# AI Agent Instructions for Versi

## Project Overview

Versi is a native GUI application for managing Node.js versions. It currently uses [fnm](https://github.com/Schniz/fnm) (Fast Node Manager) as its backend, but the architecture is backend-agnostic — adding a new backend (e.g., nvm, volta) requires only implementing the `BackendProvider` and `VersionManager` traits.

## Technology Stack

- **Language**: Rust (2021 edition)
- **GUI Framework**: [Iced](https://iced.rs/) 0.13 (Elm architecture)
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
│   │       ├── app/              # Iced Application implementation (mod.rs + handler modules)
│   │       │   ├── mod.rs        # Main update loop, Versi struct, message dispatch
│   │       │   ├── init.rs       # Initialization and backend detection
│   │       │   ├── environment.rs # Environment switching and loading
│   │       │   ├── onboarding.rs # Onboarding flow handlers
│   │       │   ├── operations.rs # Install/uninstall/set-default operations
│   │       │   ├── shell.rs      # Shell configuration handlers
│   │       │   ├── versions.rs   # Remote version fetching and update checks
│   │       │   ├── tray_handlers.rs # System tray event handlers
│   │       │   └── platform.rs   # Platform-specific helpers
│   │       ├── message.rs        # Message enum (Elm-style)
│   │       ├── state.rs          # Application state structs
│   │       ├── theme.rs          # Light/dark themes and styles
│   │       ├── settings.rs       # User settings persistence
│   │       ├── logging.rs        # Debug log file management
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
│   ├── versi-core/               # Shared utilities (release schedule, app updates, HideWindow)
│   │   └── src/
│   │       ├── schedule.rs       # Node.js release schedule fetching
│   │       ├── update.rs         # App update checking, GitHubRelease, version comparison
│   │       └── commands/mod.rs   # HideWindow trait + impls
│   ├── versi-fnm/                # fnm backend implementation
│   │   └── src/
│   │       ├── provider.rs       # FnmProvider - implements BackendProvider
│   │       ├── backend.rs        # FnmBackend - implements VersionManager
│   │       ├── client.rs         # FnmClient - CLI command execution
│   │       ├── version.rs        # Version parsing
│   │       ├── progress.rs       # Install progress tracking
│   │       ├── detection.rs      # fnm binary detection
│   │       ├── update.rs         # fnm update checking
│   │       └── error.rs          # Error types
│   ├── versi-shell/              # Shell detection & configuration (backend-agnostic)
│   │   └── src/
│   │       ├── detect.rs         # Shell detection
│   │       ├── config.rs         # Config file editing (parameterized on marker/label)
│   │       ├── shells/           # Shell-specific implementations
│   │       └── verify.rs         # Configuration verification (parameterized on marker/backend_binary)
│   └── versi-platform/           # Platform abstractions
│       └── src/
│           ├── paths.rs          # Platform-native paths
│           ├── environment.rs    # Environment abstraction
│           └── wsl.rs            # WSL distro detection (Windows)
```

## Architecture

### Elm Architecture (Model-View-Update)

The application follows Iced's Elm-style architecture:

1. **State** (`state.rs`): Immutable application state
2. **Message** (`message.rs`): Events that can modify state
3. **Update** (`app.rs`): Handles messages and returns new state + tasks
4. **View** (`views/`): Pure functions that render state to UI

### Key Patterns

- **Tasks**: Async operations return `Task<Message>` for side effects
- **Subscriptions**: Time-based events (tick for toast timeouts)
- **Theming**: Dynamic light/dark themes based on system preference
- **Operation Queue**: Installs run concurrently (`active_installs: Vec<Operation>`), while uninstall and set-default are exclusive (`exclusive_op: Option<Operation>`). Pending operations are queued and drained when capacity is available.
- **System Tray**: Optional background tray icon with version switching support
- **Backend Abstraction**: The `Versi` struct holds an `Arc<dyn BackendProvider>` which provides all backend-specific behavior. The GUI, shell, and platform crates have no direct dependency on any concrete backend.

## Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run with release optimizations
cargo build --release

# Check for errors without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Code Style Guidelines

- Follow standard Rust conventions (rustfmt)
- Use `thiserror` for error types
- Prefer `async/await` over callbacks
- Keep view functions pure (no side effects)
- Use meaningful message names that describe the event
- Group related functionality into separate crates

## UI Guidelines

- **Toasts are only for background errors.** Never use toasts (`Toast::error`) for feedback that can be shown reactively in the UI (disabled buttons, inline text, tooltips, etc.). Toasts are reserved for errors from async background operations where no other UI surface exists to report the failure (e.g., install failed, uninstall failed, set-default failed).
- Prefer disabled states with tooltips, inline status text, or view changes over transient notifications.

## Key Files to Understand

1. `crates/versi/src/app/mod.rs` - Main application logic and message dispatch
2. `crates/versi/src/state.rs` - All state types and their relationships
3. `crates/versi/src/message.rs` - All possible application events
4. `crates/versi-backend/src/traits.rs` - `BackendProvider` and `VersionManager` trait definitions
5. `crates/versi-fnm/src/provider.rs` - `FnmProvider` (concrete backend implementation)

## Common Tasks

### Adding a New Feature

1. Add new message variant(s) to `message.rs`
2. Add state fields to `state.rs` if needed
3. Handle message in `app.rs` update function
4. Update view in appropriate `views/` file

### Adding a New Backend

1. Create a new crate (e.g., `versi-volta`), following `versi-fnm` as a reference
2. Implement `BackendProvider` trait (detection, installation, update checking)
3. Implement `VersionManager` trait (list installed/remote, install, uninstall, set default)
4. Wire the new provider into `Versi::new()` in `app/mod.rs`

### Adding a New Command to the fnm Backend

1. Add method to `FnmBackend` in `versi-fnm/src/backend.rs`
2. Add any new types to `versi-backend/src/types.rs` if they're shared, or `versi-fnm/src/version.rs` if fnm-specific
3. Expose via the `VersionManager` trait if applicable
4. Create corresponding message and handler in versi

### Modifying Styles

- All styles are in `crates/versi/src/theme.rs`
- Light/dark palettes defined at the top
- Button and container styles as functions

## Testing

- Unit tests should be in the same file as the code
- Integration tests in `tests/` directory
- Test backend interactions with mock or real backend installation

## Dependencies

Key external crates:
- `iced` - GUI framework
- `tokio` - Async runtime
- `reqwest` - HTTP client (for release schedule)
- `serde` - Serialization
- `open` - Opening URLs in browser
- `dirs` - Platform directories
- `which` - Finding executables

## Data & Storage

**Settings Location:**
- macOS: `~/Library/Application Support/versi/`
- Windows: `%APPDATA%/versi/`
- Linux: `~/.config/versi/` (XDG-compliant)

**Cached Data:**
- Available Node versions list (fetched from nodejs.org)
- Node.js release schedule (from GitHub)

## Backend Interaction

The GUI interacts with backends exclusively through the `BackendProvider` and `VersionManager` traits defined in `versi-backend`. The current backend (fnm) executes CLI commands as subprocesses via `FnmClient`.

- All operations run as async tasks, keeping the UI responsive via Iced's `Task` system
- Parse stdout/stderr for status and results
- Multiple installs can run concurrently; uninstall and set-default wait for all installs to finish

**Key fnm commands used (in versi-fnm):**
- `fnm list` - Get installed versions
- `fnm list-remote` - Get available versions
- `fnm install <version>` - Install a version
- `fnm uninstall <version>` - Remove a version
- `fnm default <version>` - Set default version
- `fnm current` - Get currently active version

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
- Detects backend binary path by checking common locations (`~/.local/share/fnm/fnm`, `~/.cargo/bin/fnm`, etc.)
- Shows all distros as tabs; non-running or backend-less distros appear disabled with reason
- Commands executed directly via `wsl.exe -d <distro> /path/to/backend ...` (no shell needed)
- Shell detection in settings is environment-aware: shows Linux shells (bash/zsh/fish) for WSL environments

### Linux
- Native x64 and ARM64 binaries
- XDG-compliant paths
- Support for bash, zsh, fish shells
