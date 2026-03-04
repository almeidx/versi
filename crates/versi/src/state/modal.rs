#[derive(Debug, Clone)]
pub enum Modal {
    ConfirmBulkUpdateMajors {
        versions: Vec<(String, String)>,
    },
    ConfirmBulkUninstallEOL {
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajor {
        major: u32,
        versions: Vec<String>,
    },
    ConfirmBulkUninstallMajorExceptLatest {
        major: u32,
        versions: Vec<String>,
        keeping: String,
    },
    ConfirmUninstallDefault {
        version: String,
    },
    KeyboardShortcuts,
    VersionDetail {
        version: String,
    },
}
