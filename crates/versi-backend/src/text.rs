#[must_use]
pub fn sanitize_terminal_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => {
                if chars.peek() == Some(&'[') {
                    let _ = chars.next();
                    for esc_ch in chars.by_ref() {
                        if esc_ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            '\u{8}' => {
                let _ = output.pop();
            }
            '\n' | '\r' => output.push(ch),
            control if control.is_control() => {}
            _ => output.push(ch),
        }
    }

    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::sanitize_terminal_text;

    #[test]
    fn removes_ansi_escape_sequences() {
        let input = "\x1b[32m->     v20.11.0\x1b[0m";
        assert_eq!(sanitize_terminal_text(input), "->     v20.11.0");
    }

    #[test]
    fn passes_through_plain_text() {
        assert_eq!(sanitize_terminal_text("v20.11.0"), "v20.11.0");
    }

    #[test]
    fn handles_backspaces_and_control_chars() {
        let raw = "\u{1b}[2K^D\u{8}\u{8}00:00:00 █ 10.49 MiB/19.66 MiB (4.23 MiB/s, 2s)\r";
        assert_eq!(
            sanitize_terminal_text(raw),
            "00:00:00 █ 10.49 MiB/19.66 MiB (4.23 MiB/s, 2s)"
        );
    }
}
