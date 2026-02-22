use crate::render::Viewport;

pub fn media_query_matches(media: &str, viewport: Viewport) -> bool {
    let media = media.trim();
    if media.is_empty() {
        return true;
    }

    split_commas(media)
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() { None } else { Some(part) }
        })
        .any(|part| media_query_part_matches(part, viewport))
}

fn media_query_part_matches(part: &str, viewport: Viewport) -> bool {
    let mut scanner = Scanner::new(part);
    let mut has_any_condition = false;

    while scanner.skip_whitespace() {
        if scanner.consume_keyword("and") {
            continue;
        }

        if scanner.consume_keyword("not") {
            // Negated queries are currently unsupported.
            return false;
        }

        if scanner.consume_keyword("only") {
            continue;
        }

        if scanner.peek_char() == Some('(') {
            has_any_condition = true;
            let Some(expr) = scanner.consume_parenthesized() else {
                return false;
            };
            if !media_expression_matches(expr, viewport) {
                return false;
            }
            continue;
        }

        let Some(word) = scanner.consume_word() else {
            break;
        };
        has_any_condition = true;
        if !media_type_matches(word) {
            return false;
        }
    }

    has_any_condition
}

fn media_type_matches(token: &str) -> bool {
    matches!(token.trim().to_ascii_lowercase().as_str(), "all" | "screen")
}

fn media_expression_matches(expr: &str, viewport: Viewport) -> bool {
    let mut parts = expr.splitn(2, ':');
    let feature = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let value = parts.next().unwrap_or("").trim();

    match feature.as_str() {
        "min-width" => match parse_length_px(value) {
            Some(px) => viewport.width_px as f32 >= px,
            None => false,
        },
        "max-width" => match parse_length_px(value) {
            Some(px) => viewport.width_px as f32 <= px,
            None => false,
        },
        _ => false,
    }
}

fn parse_length_px(input: &str) -> Option<f32> {
    let value = input.trim();
    let value = value.strip_suffix("px").unwrap_or(value).trim();
    value.parse::<f32>().ok()
}

fn split_commas(input: &str) -> impl Iterator<Item = &str> {
    CommaSplitter { input, cursor: 0 }
}

struct CommaSplitter<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Iterator for CommaSplitter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.input.len() {
            return None;
        }

        let start = self.cursor;
        let bytes = self.input.as_bytes();
        let mut depth = 0usize;

        while self.cursor < bytes.len() {
            match bytes[self.cursor] {
                b'(' => depth = depth.saturating_add(1),
                b')' => depth = depth.saturating_sub(1),
                b',' if depth == 0 => {
                    let end = self.cursor;
                    self.cursor += 1;
                    return Some(self.input[start..end].trim());
                }
                _ => {}
            }
            self.cursor += 1;
        }

        Some(self.input[start..].trim())
    }
}

struct Scanner<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    fn skip_whitespace(&mut self) -> bool {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        self.cursor < self.input.len()
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let rest = &self.input[self.cursor..];
        if !rest
            .get(0..keyword.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
        {
            return false;
        }

        let after = rest.get(keyword.len()..).unwrap_or("");
        if after
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            return false;
        }

        self.cursor += keyword.len();
        true
    }

    fn consume_word(&mut self) -> Option<&'a str> {
        let start = self.cursor;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || ch == '(' || ch == ')' || ch == ',' {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        let word = self.input[start..self.cursor].trim();
        if word.is_empty() { None } else { Some(word) }
    }

    fn consume_parenthesized(&mut self) -> Option<&'a str> {
        if self.peek_char() != Some('(') {
            return None;
        }
        self.cursor += 1;
        let start = self.cursor;
        let mut depth = 1usize;

        while let Some(ch) = self.peek_char() {
            self.cursor += ch.len_utf8();
            match ch {
                '(' => depth = depth.saturating_add(1),
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let end = self.cursor.saturating_sub(1);
                        return Some(self.input[start..end].trim());
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_empty_media_as_true() {
        assert!(media_query_matches(
            "",
            Viewport {
                width_px: 10,
                height_px: 10
            }
        ));
    }

    #[test]
    fn matches_simple_min_width() {
        assert!(media_query_matches(
            "all and (min-width: 728px)",
            Viewport {
                width_px: 1024,
                height_px: 10
            }
        ));
        assert!(!media_query_matches(
            "all and (min-width: 1080px)",
            Viewport {
                width_px: 1024,
                height_px: 10
            }
        ));
    }

    #[test]
    fn matches_simple_max_width() {
        assert!(media_query_matches(
            "all and (max-width: 1079.98px)",
            Viewport {
                width_px: 1024,
                height_px: 10
            }
        ));
        assert!(!media_query_matches(
            "all and (max-width: 903.98px)",
            Viewport {
                width_px: 1024,
                height_px: 10
            }
        ));
    }

    #[test]
    fn supports_comma_separated_or_queries() {
        assert!(media_query_matches(
            "all and (min-width: 2000px), all and (max-width: 100px), screen",
            Viewport {
                width_px: 1024,
                height_px: 10
            }
        ));
    }
}
