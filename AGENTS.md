# Versi

Versi is a native Rust/Iced application for managing Node.js versions through
multiple backends. The Cargo workspace separates the GUI, backend contracts,
backend implementations, shell integration, and platform support.

Use `README.md` for build/setup instructions and the workspace manifests for
current dependencies. Keep this file focused on architectural contracts that
are easy to break across crates.

## Workspace boundaries

- `crates/versi-backend` defines `BackendProvider`, `VersionManager`, and shared
  backend types. GUI, shell, and platform code must depend on these abstractions,
  not a concrete backend crate.
- `crates/versi-{fnm,nvm,volta,asdf}` contain backend-specific detection,
  commands, parsing, and update behavior.
- `crates/versi-shell` owns shell detection, configuration, and verification.
  Keep it parameterized by backend-specific labels, markers, and binaries.
- `crates/versi-platform` owns native paths, command behavior, environments,
  and WSL support.
- `crates/versi-core` owns backend-independent remote metadata and application
  update logic.
- `crates/versi` owns the Iced application. State lives under `state/`, events
  in `message.rs`, update handling under `app/`, and rendering under `views/`.

When adding a backend, implement the backend traits, add its `BackendKind`, wire
provider construction/detection, and cover any settings, shell, environment,
and UI surfaces that enumerate supported backends. Use an existing backend as a
behavioral reference without leaking its command syntax into shared crates.

## Application invariants

- Blocking process, filesystem, and network work must run outside view code and
  return through Iced `Task<Message>` flows. Views remain deterministic
  renderings of state.
- Installs may run concurrently. Uninstall, set-default, and other exclusive
  operations must continue to coordinate through the operation queue rather
  than bypassing it.
- Preserve typed errors through backend, shell, platform, and application
  layers. Avoid introducing string-only domain errors or erasing context with
  `to_string()` before the UI/logging boundary.
- Settings changes must preserve migrations and unknown/legacy-safe behavior.
  Shell configuration writes must retain their marker and verification
  safeguards.
- Environment-aware behavior must distinguish native and WSL targets. Do not
  start stopped WSL distributions merely to probe them, and do not assume native
  paths or shells apply inside WSL.
- Use inline state, disabled controls, or contextual text for actionable UI
  feedback. Reserve error toasts for asynchronous failures without a stable UI
  surface.
- Keep platform-specific code behind existing modules and `cfg` boundaries;
  avoid spreading OS checks through shared application logic.

## Validation

Run targeted crate tests while iterating. Before completing cross-crate or
application changes, use the same core gates as CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Run ignored backend integration tests only when changing the corresponding real
backend interaction; they can require installed tools or external access.

`CLAUDE.md` is a symlink to this file. Edit `AGENTS.md` and preserve the symlink.
