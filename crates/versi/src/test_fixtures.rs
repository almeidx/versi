use versi_core::ReleaseSchedule;

pub fn schedule_with_eol_major(eol_major: u32) -> ReleaseSchedule {
    serde_json::from_value(serde_json::json!({
        "versions": {
            format!("{eol_major}"): { "start": "2020-01-01", "end": "2021-01-01" },
            "22": { "start": "2024-04-23", "lts": "2024-10-29", "maintenance": "2026-10-20", "end": "2027-04-30", "codename": "Jod" }
        }
    }))
    .expect("schedule fixture should deserialize")
}

pub fn remote(version: &str, lts_codename: Option<&str>) -> versi_backend::RemoteVersion {
    versi_backend::RemoteVersion {
        version: version.parse().expect("test version should parse"),
        lts_codename: lts_codename.map(str::to_string),
        is_latest: false,
    }
}
