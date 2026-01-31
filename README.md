# Versi

A native GUI application for managing Node.js versions.

![Versi screenshot](assets/screenshot.png)

## Features

- View and manage installed Node.js versions
- Install/uninstall Node.js versions
- Set default Node.js version
- Bulk operations: update all majors, clean EOL versions, keep only latest per major
- Check for updates and install them
- Light and dark theme support (follows system preference)
- Shell configuration detection and setup
- Search and filter versions
- System tray support with quick version switching
- WSL integration on Windows (manage Node.js in WSL distros)

## Installation

### Download Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/almeidx/versi/releases) page.

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `versi-macos-arm64.zip` |
| macOS (Intel) | `versi-macos-x64.zip` |
| Windows (x64) | `versi-windows-x64.msi` |
| Linux (x64) | `versi-linux-x64.zip` |
| Linux (ARM64) | `versi-linux-arm64.zip` |

### macOS Installation

1. Download the appropriate `.zip` file for your Mac
2. Extract the zip file
3. Run the install script (recommended):
   ```bash
   ./install.sh
   ```
   This will remove the quarantine attribute and move `Versi.app` to your Applications folder.

   **Or install manually:**
   - Drag `Versi.app` to your Applications folder
   - On first launch, macOS may block the app because it's not signed. To fix this:
     ```bash
     xattr -cr "/Applications/Versi.app"
     ```
     Or right-click the app and select "Open" to bypass Gatekeeper.

### Windows Installation

1. Download `versi-windows-x64.msi`
2. Double-click to run the installer
3. The app will be available in your Start Menu

### Linux Installation

1. Download the appropriate `.zip` file
2. Extract the archive:
   ```bash
   unzip versi-linux-x64.zip
   ```
3. Move the binary to a location in your PATH:
   ```bash
   sudo mv versi /usr/local/bin/
   ```
4. (Optional) Install the desktop entry for application launchers:
   ```bash
   mv versi.desktop ~/.local/share/applications/
   ```

### Build from Source

#### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- [fnm](https://github.com/Schniz/fnm) installed and configured

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/almeidx/versi.git
cd versi

# Build in release mode
cargo build --release

# The binary will be at target/release/versi
```

## Usage

1. **First Launch**: If fnm is not detected, the app will guide you through installation and shell configuration.

2. **Main View**: Shows all installed Node.js versions grouped by major version. Click a group to expand/collapse.

3. **Install**: Click the "Install" button to browse and install new versions. Recommended versions are shown at the top.

4. **Set Default**: Click "Set Default" on any version to make it the default.

5. **Uninstall**: Click "Uninstall" to remove a version. A toast notification appears with an "Undo" option.

6. **Updates**: If a newer version is available for an installed major version, an update badge appears. Click it to install.

7. **Settings**: Access theme preferences and shell configuration status.

## Development

### Project Structure

```
versi/
├── crates/
│   ├── versi/          # Main GUI application
│   ├── versi-core/     # fnm CLI wrapper library
│   ├── versi-shell/    # Shell detection & configuration
│   └── versi-platform/ # Platform abstractions
```

### Commands

```bash
# Run in development mode
cargo run

# Run tests
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

### Architecture

The application uses [Iced](https://iced.rs/) with the Elm architecture:
- **State**: Immutable application state
- **Message**: Events that trigger state changes
- **Update**: Logic to handle messages and produce side effects
- **View**: Pure functions rendering state to UI

See [AGENTS.md](AGENTS.md) for detailed development documentation.

## Requirements

- **fnm**: The application requires fnm to be installed. If not found, the onboarding wizard will help you install it.
- **Shell Configuration**: fnm needs to be configured in your shell for full functionality.

## License

GNU General Public License v3.0 - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please read the contributing guidelines before submitting a PR.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Acknowledgments

- [fnm](https://github.com/Schniz/fnm) - The fast Node.js version manager this UI wraps
- [Iced](https://iced.rs/) - The Rust GUI framework powering this application
