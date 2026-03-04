use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Fnm,
    Nvm,
    Volta,
    Asdf,
}

impl BackendKind {
    pub const DEFAULT: Self = Self::Fnm;

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fnm => "fnm",
            Self::Nvm => "nvm",
            Self::Volta => "volta",
            Self::Asdf => "asdf",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "fnm" => Some(Self::Fnm),
            "nvm" => Some(Self::Nvm),
            "volta" => Some(Self::Volta),
            "asdf" => Some(Self::Asdf),
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
        assert_eq!(BackendKind::Asdf.as_str(), "asdf");
        assert_eq!(BackendKind::Volta.as_str(), "volta");
    }

    #[test]
    fn from_name_accepts_known_backend_names() {
        assert_eq!(BackendKind::from_name("fnm"), Some(BackendKind::Fnm));
        assert_eq!(BackendKind::from_name("nvm"), Some(BackendKind::Nvm));
        assert_eq!(BackendKind::from_name("asdf"), Some(BackendKind::Asdf));
        assert_eq!(BackendKind::from_name("volta"), Some(BackendKind::Volta));
        assert_eq!(BackendKind::from_name("FNM"), None);
    }

    #[test]
    fn display_outputs_backend_name() {
        assert_eq!(BackendKind::Fnm.to_string(), "fnm");
        assert_eq!(BackendKind::Nvm.to_string(), "nvm");
        assert_eq!(BackendKind::Asdf.to_string(), "asdf");
        assert_eq!(BackendKind::Volta.to_string(), "volta");
    }

    #[test]
    fn as_str_roundtrips_through_from_name() {
        let all_kinds = [
            BackendKind::Fnm,
            BackendKind::Nvm,
            BackendKind::Volta,
            BackendKind::Asdf,
        ];
        for kind in all_kinds {
            assert_eq!(
                BackendKind::from_name(kind.as_str()),
                Some(kind),
                "{kind:?}.as_str() does not round-trip through from_name"
            );
        }
    }

    #[test]
    fn provider_names_match_backend_kind() {
        use versi_backend::BackendProvider;

        let providers: Vec<(&str, BackendKind)> = vec![
            (versi_fnm::FnmProvider::new().name(), BackendKind::Fnm),
            (versi_nvm::NvmProvider::new().name(), BackendKind::Nvm),
            (
                versi_volta::VoltaProvider::new(reqwest::Client::new()).name(),
                BackendKind::Volta,
            ),
            (
                versi_asdf::AsdfProvider::new(reqwest::Client::new()).name(),
                BackendKind::Asdf,
            ),
        ];

        for (provider_name, expected_kind) in providers {
            assert_eq!(
                BackendKind::from_name(provider_name),
                Some(expected_kind),
                "provider name '{provider_name}' does not map to {expected_kind:?}"
            );
        }
    }

    #[test]
    fn serde_roundtrip_supports_new_backends() {
        let volta = serde_json::to_string(&BackendKind::Volta)
            .expect("volta backend kind should serialize");
        let asdf =
            serde_json::to_string(&BackendKind::Asdf).expect("asdf backend kind should serialize");
        let volta_decoded: BackendKind =
            serde_json::from_str(&volta).expect("volta backend kind should deserialize");
        let asdf_decoded: BackendKind =
            serde_json::from_str(&asdf).expect("asdf backend kind should deserialize");

        assert_eq!(volta, "\"volta\"");
        assert_eq!(asdf, "\"asdf\"");
        assert_eq!(volta_decoded, BackendKind::Volta);
        assert_eq!(asdf_decoded, BackendKind::Asdf);
    }
}
