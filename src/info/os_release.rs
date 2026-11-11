pub(super) fn parse_value(value: &str) -> String {
    let value = value.trim();

    if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return unescape_double_quoted(inner);
    }

    if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        // Single quotes preserve every character, including backslashes.
        return inner.to_owned();
    }

    unescape_unquoted(value)
}

fn unescape_double_quoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some(next @ ('\\' | '"' | '$' | '`')) => output.push(next),
                Some(next) => {
                    output.push('\\');
                    output.push(next);
                }
                None => output.push('\\'),
            }
        } else {
            output.push(ch);
        }
    }

    output
}

fn unescape_unquoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some(next) => output.push(next),
                None => output.push('\\'),
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quotes_and_escapes_by_os_release_context() {
        assert_eq!(parse_value("\"CachyOS 2026\""), "CachyOS 2026");
        assert_eq!(parse_value("'CachyOS'"), "CachyOS");
        assert_eq!(parse_value("\"A\\\"B\""), "A\"B");
        assert_eq!(parse_value("\"A\\qB\""), "A\\qB");
        assert_eq!(parse_value("\"A\\$B\""), "A$B");
        assert_eq!(parse_value("'A\\\\B'"), "A\\\\B");
        assert_eq!(parse_value("A=\\$B"), "A=$B");
        assert_eq!(parse_value("A=\\\\B"), "A=\\B");
    }
}
