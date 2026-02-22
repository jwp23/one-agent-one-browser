pub fn supports_condition_matches(condition: &str) -> bool {
    eval_expression(condition.trim())
}

fn eval_expression(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() {
        return false;
    }

    let input = strip_outer_parentheses(input);
    if input.is_empty() {
        return false;
    }

    if let Some(rest) = strip_leading_keyword(input, "not") {
        return !eval_expression(rest);
    }

    let or_parts = split_top_level_keyword(input, "or");
    if or_parts.len() > 1 {
        return or_parts.into_iter().any(eval_expression);
    }

    let and_parts = split_top_level_keyword(input, "and");
    if and_parts.len() > 1 {
        return and_parts.into_iter().all(eval_expression);
    }

    if let Some(inner) = wrapped_parenthesized(input) {
        return eval_expression(inner);
    }

    declaration_supported(input)
}

fn strip_outer_parentheses(input: &str) -> &str {
    let mut current = input.trim();
    while let Some(inner) = wrapped_parenthesized(current) {
        current = inner.trim();
    }
    current
}

fn wrapped_parenthesized(input: &str) -> Option<&str> {
    let input = input.trim();
    if !(input.starts_with('(') && input.ends_with(')')) {
        return None;
    }

    let mut depth = 0usize;
    let mut close_at_end = false;
    for (byte_idx, ch) in input.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close_at_end = byte_idx + ch.len_utf8() == input.len();
                    if !close_at_end {
                        return None;
                    }
                }
            }
            _ => {}
        }
    }

    if depth != 0 || !close_at_end {
        return None;
    }

    Some(&input[1..input.len().saturating_sub(1)])
}

fn strip_leading_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    if !starts_with_keyword(input, keyword) {
        return None;
    }
    let rest = input.get(keyword.len()..)?.trim_start();
    if rest.is_empty() {
        return None;
    }
    Some(rest)
}

fn starts_with_keyword(input: &str, keyword: &str) -> bool {
    if input.len() < keyword.len() {
        return false;
    }
    let prefix = &input[..keyword.len()];
    if !prefix.eq_ignore_ascii_case(keyword) {
        return false;
    }
    input
        .get(keyword.len()..)
        .and_then(|tail| tail.chars().next())
        .map_or(true, |ch| ch.is_whitespace() || ch == '(')
}

fn split_top_level_keyword<'a>(input: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut cursor = 0usize;
    let bytes = input.as_bytes();
    let keyword_len = keyword.len();
    let mut start = 0usize;

    while cursor < bytes.len() {
        let ch = input[cursor..]
            .chars()
            .next()
            .expect("cursor always valid char boundary");
        match ch {
            '(' => {
                depth = depth.saturating_add(1);
                cursor += ch.len_utf8();
                continue;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cursor += ch.len_utf8();
                continue;
            }
            _ => {}
        }

        if depth == 0
            && cursor + keyword_len <= bytes.len()
            && input[cursor..cursor + keyword_len].eq_ignore_ascii_case(keyword)
        {
            let before_ok = if cursor == 0 {
                true
            } else {
                input[..cursor]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_whitespace() || c == ')')
            };
            let after_ok = if cursor + keyword_len == bytes.len() {
                true
            } else {
                input[cursor + keyword_len..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_whitespace() || c == '(')
            };
            if before_ok && after_ok {
                parts.push(input[start..cursor].trim());
                cursor += keyword_len;
                start = cursor;
                continue;
            }
        }

        cursor += ch.len_utf8();
    }

    if start == 0 {
        return vec![input.trim()];
    }
    parts.push(input[start..].trim());
    parts
}

fn declaration_supported(input: &str) -> bool {
    let Some(colon) = input.find(':') else {
        return false;
    };
    let name = input[..colon].trim().to_ascii_lowercase();
    let value = input[colon + 1..].trim().to_ascii_lowercase();
    if name.is_empty() || value.is_empty() {
        return false;
    }

    match (name.as_str(), value.as_str()) {
        ("display", "grid" | "inline-grid" | "flex" | "inline-flex" | "block" | "inline-block") => {
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::supports_condition_matches;

    #[test]
    fn supports_display_grid() {
        assert!(supports_condition_matches("(display:grid)"));
        assert!(supports_condition_matches("( display: grid )"));
    }

    #[test]
    fn supports_not_and_or_combinations() {
        assert!(supports_condition_matches(
            "not (((-webkit-mask-image:none) or (mask-image:none)))"
        ));
        assert!(!supports_condition_matches(
            "((-webkit-mask-image:none) or (mask-image:none))"
        ));
    }

    #[test]
    fn supports_and_expressions() {
        assert!(supports_condition_matches(
            "(display:grid) and (display:inline-block)"
        ));
        assert!(!supports_condition_matches(
            "(display:grid) and (mask-image:none)"
        ));
    }
}
