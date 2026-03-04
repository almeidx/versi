pub(crate) fn response_snippet(body: &str, max_chars: usize) -> String {
    let snippet: String = body.chars().take(max_chars).collect();
    if snippet.is_empty() {
        String::new()
    } else {
        format!(": {snippet}")
    }
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
