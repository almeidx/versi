use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Fnm,
    Nvm,
    Volta,
}

impl BackendKind {
    pub const DEFAULT: Self = Self::Fnm;

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fnm => "fnm",
            Self::Nvm => "nvm",
            Self::Volta => "volta",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "fnm" => Some(Self::Fnm),
            "nvm" => Some(Self::Nvm),
            "volta" => Some(Self::Volta),
            _ => None,
        }
    }
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::BackendKind;

    #[test]
    fn default_backend_is_fnm() {
        assert_eq!(BackendKind::DEFAULT, BackendKind::Fnm);
    }

    #[test]
    fn as_str_matches_expected_backend_names() {
        assert_eq!(BackendKind::Fnm.as_str(), "fnm");
        assert_eq!(BackendKind::Nvm.as_str(), "nvm");
        assert_eq!(BackendKind::Volta.as_str(), "volta");
    }

    #[test]
    fn from_name_accepts_known_backend_names() {
        assert_eq!(BackendKind::from_name("fnm"), Some(BackendKind::Fnm));
        assert_eq!(BackendKind::from_name("nvm"), Some(BackendKind::Nvm));
        assert_eq!(BackendKind::from_name("volta"), Some(BackendKind::Volta));
        assert_eq!(BackendKind::from_name("FNM"), None);
    }

    #[test]
    fn display_outputs_backend_name() {
        assert_eq!(BackendKind::Fnm.to_string(), "fnm");
        assert_eq!(BackendKind::Nvm.to_string(), "nvm");
        assert_eq!(BackendKind::Volta.to_string(), "volta");
    }

    #[test]
    fn serde_roundtrip_supports_volta() {
        let encoded =
            serde_json::to_string(&BackendKind::Volta).expect("backend kind should serialize");
        let decoded: BackendKind =
            serde_json::from_str(&encoded).expect("backend kind should deserialize");

        assert_eq!(encoded, "\"volta\"");
        assert_eq!(decoded, BackendKind::Volta);
    }
}
