pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format_tenths(bytes, GB, "GB")
    } else if bytes >= MB {
        format_tenths(bytes, MB, "MB")
    } else if bytes >= KB {
        format_tenths(bytes, KB, "KB")
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn format_tenths(value: u64, unit: u64, suffix: &str) -> String {
    let scaled = (u128::from(value) * 10 + u128::from(unit) / 2) / u128::from(unit);
    let whole = scaled / 10;
    let tenth = scaled % 10;
    format!("{whole}.{tenth} {suffix}")
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, format_tenths};

    #[test]
    fn format_tenths_zero_value() {
        assert_eq!(format_tenths(0, 1024, "KB"), "0.0 KB");
    }

    #[test]
    fn format_tenths_exact_unit() {
        assert_eq!(format_tenths(1024, 1024, "KB"), "1.0 KB");
        assert_eq!(format_tenths(2048, 1024, "KB"), "2.0 KB");
    }

    #[test]
    fn format_tenths_rounds_to_nearest_tenth() {
        assert_eq!(format_tenths(1280, 1024, "KB"), "1.3 KB");
        assert_eq!(format_tenths(1228, 1024, "KB"), "1.2 KB");
    }

    #[test]
    fn format_tenths_rounds_up_at_midpoint() {
        assert_eq!(format_tenths(1536, 1024, "KB"), "1.5 KB");
    }

    #[test]
    fn format_bytes_uses_bytes_for_small_values() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
    }

    #[test]
    fn format_bytes_uses_kilobytes_megabytes_and_gigabytes() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }
}
