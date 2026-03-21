use crate::css::Stylesheet;
use crate::dom::Document;
use crate::url::Url;
use std::sync::Arc;

pub(super) struct UrlLoader {
    pub(super) base_url: Url,
    pub(super) pool: crate::net::FetchPool,
    pub(super) html_request_id: crate::net::RequestId,
    pub(super) html_loaded: bool,
    pub(super) html_source: Option<String>,
    pub(super) stylesheets: Vec<StylesheetSlot>,
    pub(super) scripts: Vec<ScriptSlot>,
}

impl UrlLoader {
    pub(super) fn new(base_url: Url) -> Result<UrlLoader, String> {
        let mut pool = crate::net::FetchPool::new(8).with_label("page");
        let html_request_id = pool.fetch_bytes(base_url.as_str().to_owned())?;
        Ok(UrlLoader {
            base_url,
            pool,
            html_request_id,
            html_loaded: false,
            html_source: None,
            stylesheets: Vec::new(),
            scripts: Vec::new(),
        })
    }

    pub(super) fn fetch_stylesheets(
        &mut self,
        document: &Document,
    ) -> Result<Vec<StylesheetSlot>, String> {
        let mut refs = Vec::new();
        collect_stylesheet_refs(&document.root, &self.base_url, &mut refs)?;

        let mut slots = Vec::with_capacity(refs.len());
        for reference in refs {
            match reference {
                StylesheetRef::Inline { css, media } => slots.push(StylesheetSlot::Inline {
                    stylesheet: Arc::new(Stylesheet::parse(&css)),
                    media,
                }),
                StylesheetRef::External { url, media } => {
                    let id = self.pool.fetch_bytes(url.clone())?;
                    slots.push(StylesheetSlot::External {
                        request_id: id,
                        stylesheet: None,
                        media,
                    });
                }
            }
        }

        Ok(slots)
    }

    pub(super) fn fetch_scripts(&mut self, document: &Document) -> Result<Vec<ScriptSlot>, String> {
        let refs = collect_script_refs(document, &self.base_url)?;

        let mut slots = Vec::with_capacity(refs.len());
        for reference in refs {
            match reference {
                ScriptRef::Inline { source } => slots.push(ScriptSlot::Inline { source }),
                ScriptRef::External { url } => {
                    let id = self.pool.fetch_bytes(url)?;
                    slots.push(ScriptSlot::External {
                        request_id: id,
                        source: None,
                    });
                }
            }
        }

        Ok(slots)
    }

    pub(super) fn build_document(&self) -> Result<Option<Document>, String> {
        let Some(html_source) = self.html_source.as_deref() else {
            return Ok(None);
        };

        let mut document = crate::html::parse_document(html_source);
        let mut sources = Vec::new();
        for slot in &self.scripts {
            let Some(source) = slot.loaded_source() else {
                continue;
            };
            sources.push(source.to_owned());
        }
        crate::js::execute_script_sources(&mut document, &sources);
        Ok(Some(document))
    }

    pub(super) fn ready_for_screenshot(&self) -> bool {
        if !self.html_loaded {
            return false;
        }
        self.stylesheets.iter().all(|slot| slot.is_loaded())
            && self.scripts.iter().all(|slot| slot.is_loaded())
    }
}

pub(super) enum StylesheetSlot {
    Inline {
        stylesheet: Arc<Stylesheet>,
        media: Option<String>,
    },
    External {
        request_id: crate::net::RequestId,
        stylesheet: Option<Arc<Stylesheet>>,
        media: Option<String>,
    },
}

impl StylesheetSlot {
    pub(super) fn request_id(&self) -> Option<crate::net::RequestId> {
        match self {
            StylesheetSlot::Inline { .. } => None,
            StylesheetSlot::External { request_id, .. } => Some(*request_id),
        }
    }

    pub(super) fn set_stylesheet(&mut self, stylesheet: Arc<Stylesheet>) {
        match self {
            StylesheetSlot::Inline { .. } => {}
            StylesheetSlot::External {
                stylesheet: slot_sheet,
                ..
            } => {
                *slot_sheet = Some(stylesheet);
            }
        }
    }

    fn is_loaded(&self) -> bool {
        match self {
            StylesheetSlot::Inline { .. } => true,
            StylesheetSlot::External { stylesheet, .. } => stylesheet.is_some(),
        }
    }
}

pub(super) enum ScriptSlot {
    Inline {
        source: String,
    },
    External {
        request_id: crate::net::RequestId,
        source: Option<String>,
    },
}

impl ScriptSlot {
    pub(super) fn request_id(&self) -> Option<crate::net::RequestId> {
        match self {
            ScriptSlot::Inline { .. } => None,
            ScriptSlot::External { request_id, .. } => Some(*request_id),
        }
    }

    pub(super) fn set_source(&mut self, source: String) {
        match self {
            ScriptSlot::Inline { .. } => {}
            ScriptSlot::External {
                source: slot_source,
                ..
            } => {
                *slot_source = Some(source);
            }
        }
    }

    fn is_loaded(&self) -> bool {
        match self {
            ScriptSlot::Inline { .. } => true,
            ScriptSlot::External { source, .. } => source.is_some(),
        }
    }

    fn loaded_source(&self) -> Option<&str> {
        match self {
            ScriptSlot::Inline { source } => Some(source.as_str()),
            ScriptSlot::External { source, .. } => source.as_deref(),
        }
    }
}

pub(super) fn stylesheet_sources_from_loader(
    slots: &[StylesheetSlot],
) -> Vec<super::StylesheetSource> {
    let mut out = Vec::new();
    for slot in slots {
        match slot {
            StylesheetSlot::Inline { stylesheet, media } => out.push(super::StylesheetSource {
                stylesheet: stylesheet.clone(),
                media: media.clone(),
            }),
            StylesheetSlot::External {
                stylesheet: Some(stylesheet),
                media,
                ..
            } => out.push(super::StylesheetSource {
                stylesheet: stylesheet.clone(),
                media: media.clone(),
            }),
            StylesheetSlot::External {
                stylesheet: None, ..
            } => {}
        }
    }
    out
}

enum StylesheetRef {
    Inline { css: String, media: Option<String> },
    External { url: String, media: Option<String> },
}

enum ScriptRef {
    Inline { source: String },
    External { url: String },
}

fn collect_stylesheet_refs(
    element: &crate::dom::Element,
    base_url: &Url,
    out: &mut Vec<StylesheetRef>,
) -> Result<(), String> {
    if element.name == "style" {
        let mut css = String::new();
        for child in &element.children {
            if let crate::dom::Node::Text(text) = child {
                css.push_str(text);
                css.push('\n');
            }
        }
        out.push(StylesheetRef::Inline {
            css,
            media: element.attributes.get("media").map(str::to_owned),
        });
    }

    if super::is_stylesheet_link(element) {
        if let Some(href) = element.attributes.get("href") {
            let href = href.trim();
            if !href.is_empty() {
                let url = if href.starts_with("http://") || href.starts_with("https://") {
                    href.to_owned()
                } else {
                    base_url
                        .resolve(href)
                        .ok_or_else(|| format!("Failed to resolve stylesheet URL: {href}"))?
                        .as_str()
                        .to_owned()
                };
                out.push(StylesheetRef::External {
                    url,
                    media: element.attributes.get("media").map(str::to_owned),
                });
            }
        }
    }

    for child in &element.children {
        if let crate::dom::Node::Element(el) = child {
            collect_stylesheet_refs(el, base_url, out)?;
        }
    }

    Ok(())
}

fn collect_script_refs(document: &Document, base_url: &Url) -> Result<Vec<ScriptRef>, String> {
    let mut refs = Vec::new();
    collect_script_refs_from_element(&document.root, base_url, &mut refs)?;

    let mut page_modules = Vec::new();
    let mut startup_script_url: Option<String> = None;
    for reference in &refs {
        match reference {
            ScriptRef::Inline { source } => {
                append_unique_modules(
                    &mut page_modules,
                    &parse_resource_loader_page_modules(source),
                );
            }
            ScriptRef::External { url } => {
                if startup_script_url.is_none() && is_mediawiki_startup_script_url(url) {
                    startup_script_url = Some(url.clone());
                }
            }
        }
    }

    let supported_modules = select_supported_mediawiki_modules(&page_modules);
    if !supported_modules.is_empty()
        && let Some(startup_script_url) = startup_script_url
        && let Some(url) =
            build_mediawiki_module_bundle_url(base_url, &startup_script_url, &supported_modules)
    {
        refs.push(ScriptRef::External { url });
    }

    Ok(refs)
}

fn collect_script_refs_from_element(
    element: &crate::dom::Element,
    base_url: &Url,
    out: &mut Vec<ScriptRef>,
) -> Result<(), String> {
    if element.name == "script" && is_classic_javascript_type(element.attributes.get("type")) {
        if let Some(src) = element.attributes.get("src") {
            let src = src.trim();
            if !src.is_empty() {
                let url = if src.starts_with("http://") || src.starts_with("https://") {
                    src.to_owned()
                } else {
                    base_url
                        .resolve(src)
                        .ok_or_else(|| format!("Failed to resolve script URL: {src}"))?
                        .as_str()
                        .to_owned()
                };
                out.push(ScriptRef::External { url });
            }
        } else {
            let mut source = String::new();
            for child in &element.children {
                if let crate::dom::Node::Text(text) = child {
                    source.push_str(text);
                    source.push('\n');
                }
            }
            out.push(ScriptRef::Inline { source });
        }
    }

    for child in &element.children {
        if let crate::dom::Node::Element(el) = child {
            collect_script_refs_from_element(el, base_url, out)?;
        }
    }

    Ok(())
}

fn is_classic_javascript_type(script_type: Option<&str>) -> bool {
    let Some(script_type) = script_type else {
        return true;
    };

    let script_type = script_type.trim();
    if script_type.is_empty() {
        return true;
    }

    let mime = script_type
        .split(';')
        .next()
        .unwrap_or(script_type)
        .trim()
        .to_ascii_lowercase();

    matches!(
        mime.as_str(),
        "text/javascript" | "application/javascript" | "text/ecmascript" | "application/ecmascript"
    )
}

fn parse_resource_loader_page_modules(script: &str) -> Vec<String> {
    const MARKER: &str = "RLPAGEMODULES";
    let mut out = Vec::new();
    let mut cursor = 0usize;

    while cursor < script.len() {
        let Some(offset) = script[cursor..].find(MARKER) else {
            break;
        };
        let mut pos = cursor + offset + MARKER.len();
        pos = skip_whitespace(script, pos);
        let Some(next) = consume_char(script, pos, '=') else {
            cursor = pos;
            continue;
        };
        pos = skip_whitespace(script, next);
        let Some(next) = consume_char(script, pos, '[') else {
            cursor = pos;
            continue;
        };
        pos = skip_whitespace(script, next);

        while pos < script.len() {
            if let Some(next) = consume_char(script, pos, ']') {
                cursor = next;
                break;
            }

            let Some((module_name, next)) = parse_js_string_literal(script, pos) else {
                cursor = pos.saturating_add(1);
                break;
            };
            append_unique_modules(&mut out, &[module_name]);

            pos = skip_whitespace(script, next);
            if let Some(next) = consume_char(script, pos, ',') {
                pos = skip_whitespace(script, next);
                continue;
            }
            if let Some(next) = consume_char(script, pos, ']') {
                cursor = next;
                break;
            }
            cursor = pos.saturating_add(1);
            break;
        }
    }

    out
}

fn select_supported_mediawiki_modules(page_modules: &[String]) -> Vec<String> {
    const SUPPORTED: &[&str] = &[
        "skins.vector.js",
        "ext.visualEditor.desktopArticleTarget.init",
    ];

    let mut out = Vec::new();
    for module_name in page_modules {
        if SUPPORTED.contains(&module_name.as_str()) {
            append_unique_modules(&mut out, std::slice::from_ref(module_name));
        }
    }
    out
}

fn append_unique_modules(out: &mut Vec<String>, modules: &[String]) {
    for module_name in modules {
        if !out.iter().any(|existing| existing == module_name) {
            out.push(module_name.clone());
        }
    }
}

fn is_mediawiki_startup_script_url(url: &str) -> bool {
    extract_query_param(url, "modules").as_deref() == Some("startup")
        && extract_query_param(url, "only").as_deref() == Some("scripts")
}

fn build_mediawiki_module_bundle_url(
    base_url: &Url,
    startup_script_url: &str,
    modules: &[String],
) -> Option<String> {
    let resolved = if startup_script_url.starts_with("http://")
        || startup_script_url.starts_with("https://")
    {
        Url::parse(startup_script_url).ok()?
    } else {
        base_url.resolve(startup_script_url)?
    };

    let lang = extract_query_param(resolved.as_str(), "lang")?;
    let skin = extract_query_param(resolved.as_str(), "skin")?;
    let path = resolved
        .path_and_query()
        .split('?')
        .next()
        .unwrap_or(resolved.path_and_query());
    let base = resolved
        .as_str()
        .strip_suffix(resolved.path_and_query())
        .unwrap_or(resolved.as_str());

    Some(format!(
        "{base}{path}?lang={lang}&modules={}&only=scripts&raw=1&skin={skin}",
        modules.join("%7C")
    ))
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1.split('#').next().unwrap_or("");
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(percent_decode(value));
        }
    }
    None
}

fn percent_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = &input[i + 1..i + 3];
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    out.push(value as char);
                    i += 3;
                    continue;
                }
                out.push('%');
            }
            b'+' => out.push(' '),
            byte => out.push(byte as char),
        }
        i += 1;
    }

    out
}

fn parse_js_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let quote = source[start..].chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }

    let mut out = String::new();
    let mut escaped = false;
    let mut cursor = start + quote.len_utf8();

    while cursor < source.len() {
        let ch = source[cursor..].chars().next()?;
        cursor += ch.len_utf8();

        if escaped {
            escaped = false;
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                'u' => {
                    let end = cursor.checked_add(4)?;
                    if end > source.len() {
                        return None;
                    }
                    let value = u32::from_str_radix(&source[cursor..end], 16).ok()?;
                    out.push(char::from_u32(value)?);
                    cursor = end;
                }
                _ => out.push(ch),
            }
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some((out, cursor));
        }
        out.push(ch);
    }

    None
}

fn skip_whitespace(source: &str, start: usize) -> usize {
    let mut cursor = start;
    while cursor < source.len() {
        let Some(ch) = source[cursor..].chars().next() else {
            break;
        };
        if !ch.is_ascii_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn consume_char(source: &str, start: usize, expected: char) -> Option<usize> {
    let ch = source[start..].chars().next()?;
    if ch != expected {
        return None;
    }
    Some(start + ch.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mediawiki_page_modules_from_inline_script() {
        let script = r#"
            RLSTATE={};
            RLPAGEMODULES=["mediawiki.page.ready","skins.vector.js","ext.visualEditor.desktopArticleTarget.init"];
        "#;

        assert_eq!(
            parse_resource_loader_page_modules(script),
            vec![
                "mediawiki.page.ready".to_owned(),
                "skins.vector.js".to_owned(),
                "ext.visualEditor.desktopArticleTarget.init".to_owned(),
            ]
        );
    }

    #[test]
    fn builds_mediawiki_module_bundle_url_from_startup_script() {
        let base_url = Url::parse("https://en.wikipedia.org/wiki/Riki_LeCotey").unwrap();
        let startup = "/w/load.php?lang=en&modules=startup&only=scripts&raw=1&skin=vector-2022";
        let modules = vec![
            "skins.vector.js".to_owned(),
            "ext.visualEditor.desktopArticleTarget.init".to_owned(),
        ];

        let url = build_mediawiki_module_bundle_url(&base_url, startup, &modules).unwrap();

        assert_eq!(
            url,
            "https://en.wikipedia.org/w/load.php?lang=en&modules=skins.vector.js%7Cext.visualEditor.desktopArticleTarget.init&only=scripts&raw=1&skin=vector-2022"
        );
    }
}
