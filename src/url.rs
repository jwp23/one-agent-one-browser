#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    full: String,
    scheme: Scheme,
    host: String,
    port: Option<u16>,
    path_and_query: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("URL is empty".to_owned());
        }

        let (input, _) = split_once(input, '#');
        let (scheme, rest) = input
            .split_once("://")
            .ok_or_else(|| format!("Invalid URL (missing scheme): {input}"))?;

        let scheme = match scheme.to_ascii_lowercase().as_str() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            _ => return Err(format!("Unsupported URL scheme: {scheme}")),
        };

        let authority_end = rest
            .find(|ch: char| matches!(ch, '/' | '?' | '#'))
            .unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let mut path_and_query = rest[authority_end..].to_owned();
        if path_and_query.is_empty() {
            path_and_query.push('/');
        } else if path_and_query.starts_with('?') {
            path_and_query.insert(0, '/');
        }

        let (host, port) = parse_authority(authority)?;
        let path_and_query = strip_fragment(&path_and_query);

        Ok(Self::new(scheme, host, port, path_and_query))
    }

    pub fn as_str(&self) -> &str {
        &self.full
    }

    pub fn resolve(&self, reference: &str) -> Option<Url> {
        let reference = reference.trim();
        if reference.is_empty() {
            return None;
        }

        if reference.starts_with("http://") || reference.starts_with("https://") {
            return Url::parse(reference).ok();
        }

        if let Some(rest) = reference.strip_prefix("//") {
            let scheme = self.scheme.as_str();
            return Url::parse(&format!("{scheme}://{rest}")).ok();
        }

        let reference = strip_fragment(reference);
        if reference.starts_with('/') {
            return Some(Self::new(
                self.scheme,
                self.host.clone(),
                self.port,
                reference,
            ));
        }

        let base_path = self.path_without_query();
        let base_dir = base_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        let mut joined = String::new();
        joined.push_str(base_dir);
        joined.push('/');
        joined.push_str(reference.trim_start_matches("./"));
        if !joined.starts_with('/') {
            joined.insert(0, '/');
        }

        Some(Self::new(
            self.scheme,
            self.host.clone(),
            self.port,
            &joined,
        ))
    }

    fn new(scheme: Scheme, host: String, port: Option<u16>, path_and_query: &str) -> Url {
        let mut full = String::new();
        full.push_str(scheme.as_str());
        full.push_str("://");
        full.push_str(&host);
        if let Some(port) = port {
            full.push(':');
            full.push_str(&port.to_string());
        }
        if !path_and_query.starts_with('/') {
            full.push('/');
        }
        full.push_str(path_and_query);

        Url {
            scheme,
            host,
            port,
            path_and_query: path_and_query.to_owned(),
            full,
        }
    }

    fn path_without_query(&self) -> &str {
        let (path, _) = split_once(&self.path_and_query, '?');
        path
    }
}

impl Scheme {
    fn as_str(self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }
}

fn strip_fragment(input: &str) -> &str {
    let (head, _) = split_once(input, '#');
    head
}

fn split_once<'a>(input: &'a str, delimiter: char) -> (&'a str, Option<&'a str>) {
    match input.find(delimiter) {
        Some(idx) => (&input[..idx], Some(&input[idx + delimiter.len_utf8()..])),
        None => (input, None),
    }
}

fn parse_authority(authority: &str) -> Result<(String, Option<u16>), String> {
    if authority.is_empty() {
        return Err("Invalid URL (missing host)".to_owned());
    }

    if authority.starts_with('[') {
        return Err("IPv6 URL hosts are not supported yet".to_owned());
    }

    if let Some((host, port_str)) = authority.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return Ok((host.to_owned(), Some(port)));
        }
    }

    Ok((authority.to_owned(), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_url_with_query() {
        let url = Url::parse("https://example.com/front?day=2026-01-16").unwrap();
        assert_eq!(url.as_str(), "https://example.com/front?day=2026-01-16");
    }

    #[test]
    fn resolves_relative_path_against_file_like_path() {
        let base = Url::parse("https://news.ycombinator.com/front?day=2026-01-16").unwrap();
        let resolved = base.resolve("news.css?t=abc").unwrap();
        assert_eq!(resolved.as_str(), "https://news.ycombinator.com/news.css?t=abc");
    }

    #[test]
    fn resolves_root_relative_path() {
        let base = Url::parse("https://example.com/dir/page").unwrap();
        let resolved = base.resolve("/style.css").unwrap();
        assert_eq!(resolved.as_str(), "https://example.com/style.css");
    }
}
