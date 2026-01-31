use versi_backend::{InstallPhase, InstallProgress};

pub fn parse_progress_line(line: &str) -> Option<InstallProgress> {
    let line = line.trim();

    if line.is_empty() {
        return None;
    }

    if line.starts_with("Installing Node") {
        return Some(InstallProgress {
            phase: InstallPhase::Downloading,
            ..Default::default()
        });
    }

    if line.contains("Downloading") {
        let mut progress = InstallProgress {
            phase: InstallPhase::Downloading,
            ..Default::default()
        };

        if let Some(percent) = extract_percentage(line) {
            progress.percent = Some(percent);
        }

        if let Some((downloaded, total)) = extract_bytes(line) {
            progress.bytes_downloaded = Some(downloaded);
            progress.total_bytes = Some(total);
        }

        return Some(progress);
    }

    if line.contains("Extracting") || line.contains("extract") {
        return Some(InstallProgress {
            phase: InstallPhase::Extracting,
            ..Default::default()
        });
    }

    if line.contains("Installing") {
        return Some(InstallProgress {
            phase: InstallPhase::Installing,
            ..Default::default()
        });
    }

    if line.contains("installed") || line.contains("complete") || line.contains("success") {
        return Some(InstallProgress {
            phase: InstallPhase::Complete,
            percent: Some(100.0),
            ..Default::default()
        });
    }

    None
}

fn extract_percentage(line: &str) -> Option<f32> {
    for part in line.split_whitespace() {
        if part.ends_with('%')
            && let Ok(percent) = part.trim_end_matches('%').parse::<f32>()
        {
            return Some(percent);
        }
    }
    None
}

fn extract_bytes(line: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = line.split('/').collect();
    if parts.len() >= 2 {
        let downloaded = parse_byte_size(parts[0].split_whitespace().last()?)?;
        let total = parse_byte_size(parts[1].split_whitespace().next()?)?;
        return Some((downloaded, total));
    }
    None
}

fn parse_byte_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num_str, multiplier) = if s.ends_with("GB") || s.ends_with("G") {
        (
            s.trim_end_matches("GB").trim_end_matches('G'),
            1_000_000_000,
        )
    } else if s.ends_with("MB") || s.ends_with('M') {
        (s.trim_end_matches("MB").trim_end_matches('M'), 1_000_000)
    } else if s.ends_with("KB") || s.ends_with('K') {
        (s.trim_end_matches("KB").trim_end_matches('K'), 1_000)
    } else if s.ends_with('B') {
        (s.trim_end_matches('B'), 1)
    } else {
        (s, 1)
    };

    num_str
        .trim()
        .parse::<f64>()
        .ok()
        .map(|n| (n * multiplier as f64) as u64)
}
