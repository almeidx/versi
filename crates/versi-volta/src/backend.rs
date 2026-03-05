use async_trait::async_trait;
use std::path::{Path, PathBuf};

use versi_backend::{
    BackendError, BackendInfo, CommandEnvironment, InstalledVersion, ManagerCapabilities,
    NodeVersion, RemoteVersion, ShellInitOptions, VersionManager, execute_backend_command,
    strip_version_prefix,
};

use crate::version::{
    parse_first_runtime_version, parse_installed_versions, parse_node_index_remote_versions,
};

const NODE_INDEX_URL: &str = "https://nodejs.org/dist/index.json";

#[derive(Debug, Clone)]
pub struct VoltaBackend {
    info: BackendInfo,
    command_env: CommandEnvironment,
    http_client: reqwest::Client,
}

impl VoltaBackend {
    #[must_use]
    pub fn new(
        path: PathBuf,
        version: Option<String>,
        volta_home: Option<PathBuf>,
        http_client: reqwest::Client,
    ) -> Self {
        let command_env = CommandEnvironment::Native {
            binary_path: path.clone(),
        };
        Self {
            info: BackendInfo {
                name: "volta",
                path,
                version,
                data_dir: volta_home,
                in_path: true,
            },
            command_env,
            http_client,
        }
    }

    #[must_use]
    pub fn with_wsl(distro: String, volta_path: String, http_client: reqwest::Client) -> Self {
        let volta_home = infer_home_from_binary(Path::new(&volta_path));
        Self {
            info: BackendInfo {
                name: "volta",
                path: PathBuf::from(&volta_path),
                version: None,
                data_dir: volta_home,
                in_path: false,
            },
            command_env: CommandEnvironment::Wsl {
                distro,
                binary_path: volta_path,
            },
            http_client,
        }
    }

    #[must_use]
    pub fn with_in_path(mut self, in_path: bool) -> Self {
        self.info.in_path = in_path;
        self
    }

    async fn execute(&self, args: &[&str]) -> Result<String, BackendError> {
        execute_backend_command("volta", &self.command_env, args).await
    }

    fn normalized_node_spec(version: &str) -> String {
        format!("node@{}", strip_version_prefix(version))
    }

    async fn fetch_remote_versions(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        let response = self
            .http_client
            .get(NODE_INDEX_URL)
            .header("User-Agent", versi_core::http::USER_AGENT)
            .send()
            .await
            .map_err(|error| BackendError::network_request_from("volta remote versions", error))?;

        if !response.status().is_success() {
            return Err(BackendError::network_request(
                "volta remote versions",
                format!("HTTP {}", response.status()),
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|error| BackendError::network_parse_from("volta remote versions", error))?;

        parse_node_index_remote_versions(&body)
            .map_err(|error| BackendError::network_parse_from("volta remote versions", error))
    }

    fn shell_home_for_shell(&self, _shell: &str) -> String {
        if let Some(path) = &self.info.data_dir {
            return path.to_string_lossy().to_string();
        }

        #[cfg(windows)]
        {
            if matches!(_shell, "powershell" | "pwsh") {
                "$env:LOCALAPPDATA\\Volta".to_string()
            } else {
                "$HOME/.volta".to_string()
            }
        }

        #[cfg(not(windows))]
        {
            "$HOME/.volta".to_string()
        }
    }
}

#[async_trait]
impl VersionManager for VoltaBackend {
    fn name(&self) -> &'static str {
        "volta"
    }

    fn capabilities(&self) -> ManagerCapabilities {
        ManagerCapabilities {
            supports_lts_filter: true,
            supports_use_version: false,
            supports_shell_integration: true,
            supports_auto_switch: false,
            supports_corepack: false,
            supports_resolve_engines: false,
            supports_uninstall: false,
        }
    }

    fn backend_info(&self) -> &BackendInfo {
        &self.info
    }

    async fn list_installed(&self) -> Result<Vec<InstalledVersion>, BackendError> {
        let output = self.execute(&["list", "node", "--format", "plain"]).await?;
        Ok(parse_installed_versions(&output))
    }

    async fn list_remote(&self) -> Result<Vec<RemoteVersion>, BackendError> {
        self.fetch_remote_versions().await
    }

    async fn current_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let output = self
            .execute(&["list", "--current", "node", "--format", "plain"])
            .await?;
        Ok(parse_first_runtime_version(&output))
    }

    async fn default_version(&self) -> Result<Option<NodeVersion>, BackendError> {
        let output = self
            .execute(&["list", "--default", "node", "--format", "plain"])
            .await?;
        Ok(parse_first_runtime_version(&output))
    }

    async fn install(&self, version: &str) -> Result<(), BackendError> {
        let spec = Self::normalized_node_spec(version);
        self.execute(&["fetch", &spec]).await?;
        Ok(())
    }

    async fn uninstall(&self, _version: &str) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            operation: "uninstall",
        })
    }

    async fn set_default(&self, version: &str) -> Result<(), BackendError> {
        let spec = Self::normalized_node_spec(version);
        self.execute(&["install", &spec]).await?;
        Ok(())
    }

    fn shell_init_command(&self, shell: &str, _options: &ShellInitOptions) -> Option<String> {
        let home = self.shell_home_for_shell(shell);
        match shell {
            "bash" | "zsh" => Some(format!(
                "export VOLTA_HOME=\"{home}\" && export PATH=\"$VOLTA_HOME/bin:$PATH\""
            )),
            "fish" => Some(format!(
                "set -gx VOLTA_HOME \"{home}\"; set -gx PATH \"$VOLTA_HOME/bin\" $PATH"
            )),
            "powershell" | "pwsh" => Some(format!(
                "$env:VOLTA_HOME = \"{home}\"; $env:Path = \"$env:VOLTA_HOME\\bin;$env:Path\""
            )),
            _ => None,
        }
    }
}

fn infer_home_from_binary(binary_path: &Path) -> Option<PathBuf> {
    let file_name = binary_path
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    if !file_name.starts_with("volta") {
        return None;
    }

    let bin_dir = binary_path.parent()?;
    let bin_name = bin_dir.file_name()?.to_string_lossy().to_ascii_lowercase();
    if bin_name == "bin" {
        let home = bin_dir.parent()?;
        let home_name = home.file_name()?.to_string_lossy().to_ascii_lowercase();
        if home_name == ".volta" || home_name == "volta" {
            Some(home.to_path_buf())
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use versi_backend::{BackendInfo, ManagerCapabilities, VersionManager};

    use super::{VoltaBackend, infer_home_from_binary};

    fn backend() -> VoltaBackend {
        VoltaBackend::new(
            PathBuf::from("volta"),
            Some("2.0.2".to_string()),
            Some(PathBuf::from("/Users/test/.volta")),
            reqwest::Client::new(),
        )
    }

    #[test]
    fn capabilities_reflect_volta_support_matrix() {
        let capabilities = backend().capabilities();

        assert_eq!(
            capabilities,
            ManagerCapabilities {
                supports_lts_filter: true,
                supports_use_version: false,
                supports_shell_integration: true,
                supports_auto_switch: false,
                supports_corepack: false,
                supports_resolve_engines: false,
                supports_uninstall: false,
            }
        );
    }

    #[test]
    fn shell_init_command_builds_supported_shell_commands() {
        let options = versi_backend::ShellInitOptions::default();

        let bash = backend()
            .shell_init_command("bash", &options)
            .expect("bash should be supported");
        let fish = backend()
            .shell_init_command("fish", &options)
            .expect("fish should be supported");
        let pwsh = backend()
            .shell_init_command("pwsh", &options)
            .expect("pwsh should be supported");

        assert!(bash.contains("VOLTA_HOME"));
        assert!(fish.contains("set -gx VOLTA_HOME"));
        assert!(pwsh.contains("$env:VOLTA_HOME"));
        assert!(backend().shell_init_command("nu", &options).is_none());
    }

    #[test]
    fn with_wsl_sets_backend_info_path_and_home() {
        let backend = VoltaBackend::with_wsl(
            "Ubuntu".to_string(),
            "/home/user/.volta/bin/volta".to_string(),
            reqwest::Client::new(),
        );
        let info: &BackendInfo = backend.backend_info();

        assert_eq!(info.path, PathBuf::from("/home/user/.volta/bin/volta"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/home/user/.volta")));
        assert!(!info.in_path);
    }

    #[test]
    fn infer_home_from_binary_returns_parent_of_bin_dir() {
        assert_eq!(
            infer_home_from_binary(PathBuf::from("/home/user/.volta/bin/volta").as_path()),
            Some(PathBuf::from("/home/user/.volta"))
        );
        assert_eq!(
            infer_home_from_binary(PathBuf::from("/Users/user/Library/Volta/bin/volta").as_path()),
            Some(PathBuf::from("/Users/user/Library/Volta"))
        );
        assert!(infer_home_from_binary(PathBuf::from("/usr/bin/volta").as_path()).is_none());
        assert!(infer_home_from_binary(PathBuf::from("/usr/bin/node").as_path()).is_none());
    }
}
