use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentId {
    Native,
    Wsl {
        distro: String,
        backend_path: String,
    },
}

impl EnvironmentId {
    pub fn display_name(&self) -> String {
        match self {
            EnvironmentId::Native => {
                #[cfg(target_os = "macos")]
                {
                    "macOS".to_string()
                }
                #[cfg(target_os = "windows")]
                {
                    "Windows".to_string()
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    "Linux".to_string()
                }
            }
            EnvironmentId::Wsl { distro, .. } => format!("WSL: {}", distro),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub id: EnvironmentId,
    pub name: String,
    pub enabled: bool,
}

impl Environment {
    pub fn native() -> Self {
        Self {
            id: EnvironmentId::Native,
            name: EnvironmentId::Native.display_name(),
            enabled: true,
        }
    }

    pub fn wsl(distro: String, backend_path: String) -> Self {
        let id = EnvironmentId::Wsl {
            distro: distro.clone(),
            backend_path,
        };
        Self {
            name: id.display_name(),
            id,
            enabled: true,
        }
    }
}
