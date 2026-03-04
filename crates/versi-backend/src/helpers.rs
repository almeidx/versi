use crate::error::BackendError;
use crate::types::NodeVersion;

const SENTINELS: &[&str] = &["none", "system"];

/// Parse the output of a "current version" command into a [`NodeVersion`].
///
/// Returns `Ok(None)` when the output is empty or a known sentinel
/// (`"none"`, `"system"`), indicating no active Node version.
///
/// # Errors
///
/// Returns [`BackendError::ParseError`] when the trimmed output is
/// present but cannot be parsed as a valid semver version string.
pub fn parse_current_version(output: &str) -> Result<Option<NodeVersion>, BackendError> {
    let trimmed = output.trim().strip_prefix('v').unwrap_or(output.trim());

    if trimmed.is_empty() || SENTINELS.contains(&trimmed) {
        return Ok(None);
    }

    trimmed.parse().map(Some).map_err(BackendError::from)
}

#[cfg(test)]
mod tests {
    use super::parse_current_version;
    use crate::types::NodeVersion;

    #[test]
    fn empty_string_returns_none() {
        assert_eq!(parse_current_version("").unwrap(), None);
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert_eq!(parse_current_version("   ").unwrap(), None);
    }

    #[test]
    fn none_sentinel_returns_none() {
        assert_eq!(parse_current_version("none").unwrap(), None);
    }

    #[test]
    fn system_sentinel_returns_none() {
        assert_eq!(parse_current_version("system").unwrap(), None);
    }

    #[test]
    fn padded_sentinels_return_none() {
        assert_eq!(parse_current_version("  none  ").unwrap(), None);
        assert_eq!(parse_current_version("  system  ").unwrap(), None);
    }

    #[test]
    fn valid_version_without_prefix() {
        let result = parse_current_version("20.11.0").unwrap();
        assert_eq!(result, Some(NodeVersion::new(20, 11, 0)));
    }

    #[test]
    fn valid_version_with_v_prefix() {
        let result = parse_current_version("v20.11.0").unwrap();
        assert_eq!(result, Some(NodeVersion::new(20, 11, 0)));
    }

    #[test]
    fn valid_version_with_surrounding_whitespace() {
        let result = parse_current_version("  v22.1.3  ").unwrap();
        assert_eq!(result, Some(NodeVersion::new(22, 1, 3)));
    }

    #[test]
    fn invalid_version_returns_error() {
        assert!(parse_current_version("not-a-version").is_err());
    }
}
