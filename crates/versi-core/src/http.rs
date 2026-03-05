pub const USER_AGENT: &str = concat!("Versi/", env!("CARGO_PKG_VERSION"));

pub(crate) fn response_snippet(body: &str, max_chars: usize) -> String {
    if body.is_empty() {
        return String::new();
    }

    let end = body
        .char_indices()
        .nth(max_chars)
        .map_or(body.len(), |(idx, _)| idx);

    format!(": {}", &body[..end])
}

#[cfg(test)]
mod tests {
    use super::response_snippet;

    #[test]
    fn empty_body_returns_empty_string() {
        assert_eq!(response_snippet("", 100), "");
    }

    #[test]
    fn short_body_prefixed_with_colon() {
        assert_eq!(response_snippet("Not Found", 100), ": Not Found");
    }

    #[test]
    fn long_body_is_truncated() {
        let body = "a".repeat(200);
        let result = response_snippet(&body, 10);
        assert_eq!(result, format!(": {}", "a".repeat(10)));
    }
}
