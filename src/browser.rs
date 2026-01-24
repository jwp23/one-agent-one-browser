use crate::dom::Document;
use crate::render::{DisplayCommand, DisplayList, Painter, Viewport};
use crate::style::StyleComputer;
use crate::url::Url;

pub struct BrowserApp {
    title: String,
    document: Document,
    styles: StyleComputer,
    cached_layout: Option<(Viewport, DisplayList)>,
}

impl BrowserApp {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Browser")
            .to_owned();
        let base_dir = path
            .parent()
            .map(std::path::Path::to_owned)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        Self::from_html_with_base(&title, &source, Some(ResourceBase::FileDir(base_dir)))
    }

    pub fn from_html(title: &str, html_source: &str) -> Result<Self, String> {
        Self::from_html_with_base(title, html_source, None)
    }

    pub fn from_url(url: &str) -> Result<Self, String> {
        let base_url = Url::parse(url)?;
        let html_source = crate::net::fetch_url_text(base_url.as_str())?;
        let document = crate::html::parse_document(&html_source);

        let title = document_title(&document).unwrap_or_else(|| base_url.as_str().to_owned());
        Self::from_document_with_base(&title, document, Some(ResourceBase::Url(base_url)))
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String> {
        if self
            .cached_layout
            .as_ref()
            .is_some_and(|(cached_viewport, _)| *cached_viewport == viewport)
        {
        } else {
            let display_list =
                crate::layout::layout_document(&self.document, &self.styles, painter, viewport)?;
            self.cached_layout = Some((viewport, display_list));
        }

        painter.clear()?;

        if let Some((_, list)) = &self.cached_layout {
            for cmd in &list.commands {
                match cmd {
                    DisplayCommand::Rect(rect) => painter.fill_rect(
                        rect.x_px,
                        rect.y_px,
                        rect.width_px,
                        rect.height_px,
                        rect.color,
                    )?,
                    DisplayCommand::Text(text) => {
                        painter.draw_text(text.x_px, text.y_px, &text.text, text.style)?;
                    }
                }
            }
        }

        painter.flush()?;
        Ok(())
    }
}

enum ResourceBase {
    Url(Url),
    FileDir(std::path::PathBuf),
}

impl BrowserApp {
    fn from_html_with_base(
        title: &str,
        html_source: &str,
        base: Option<ResourceBase>,
    ) -> Result<Self, String> {
        let document = crate::html::parse_document(html_source);
        Self::from_document_with_base(title, document, base)
    }

    fn from_document_with_base(
        title: &str,
        document: Document,
        base: Option<ResourceBase>,
    ) -> Result<Self, String> {
        let css = collect_page_stylesheets(&document, base.as_ref())?;
        let styles = StyleComputer::from_css(&css);
        Ok(Self {
            title: title.to_owned(),
            document,
            styles,
            cached_layout: None,
        })
    }
}

fn document_title(document: &Document) -> Option<String> {
    let title = document.find_first_element_by_name("title")?;
    let mut out = String::new();
    for child in &title.children {
        if let crate::dom::Node::Text(text) = child {
            out.push_str(text);
        }
    }
    let out = out.trim();
    if out.is_empty() {
        None
    } else {
        Some(out.to_owned())
    }
}

fn collect_page_stylesheets(document: &Document, base: Option<&ResourceBase>) -> Result<String, String> {
    let mut out = String::new();
    collect_page_stylesheets_from_element(&document.root, base, &mut out)?;
    Ok(out)
}

fn collect_page_stylesheets_from_element(
    element: &crate::dom::Element,
    base: Option<&ResourceBase>,
    out: &mut String,
) -> Result<(), String> {
    if element.name == "style" {
        for child in &element.children {
            if let crate::dom::Node::Text(text) = child {
                out.push_str(text);
                out.push('\n');
            }
        }
    }

    if is_stylesheet_link(element) {
        if let Some(href) = element.attributes.get("href") {
            if let Some(css) = load_stylesheet_text(href, base)? {
                out.push_str(&css);
                out.push('\n');
            }
        }
    }

    for child in &element.children {
        if let crate::dom::Node::Element(el) = child {
            collect_page_stylesheets_from_element(el, base, out)?;
        }
    }

    Ok(())
}

fn is_stylesheet_link(element: &crate::dom::Element) -> bool {
    if element.name != "link" {
        return false;
    }
    let Some(rel) = element.attributes.get("rel") else {
        return false;
    };
    rel.split_whitespace()
        .any(|token| token.eq_ignore_ascii_case("stylesheet"))
}

fn load_stylesheet_text(href: &str, base: Option<&ResourceBase>) -> Result<Option<String>, String> {
    let href = href.trim();
    if href.is_empty() {
        return Ok(None);
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(Some(crate::net::fetch_url_text(href)?));
    }

    let Some(base) = base else {
        return Ok(None);
    };

    match base {
        ResourceBase::Url(base_url) => {
            let url = base_url
                .resolve(href)
                .ok_or_else(|| format!("Failed to resolve stylesheet URL: {href}"))?;
            Ok(Some(crate::net::fetch_url_text(url.as_str())?))
        }
        ResourceBase::FileDir(dir) => {
            let path = resolve_stylesheet_file_path(dir, href);
            match std::fs::read_to_string(&path) {
                Ok(css) => Ok(Some(css)),
                Err(_) => Ok(None),
            }
        }
    }
}

fn resolve_stylesheet_file_path(base_dir: &std::path::Path, href: &str) -> std::path::PathBuf {
    let href = href
        .split('#')
        .next()
        .unwrap_or(href)
        .split('?')
        .next()
        .unwrap_or(href);

    if href.starts_with('/') {
        return std::path::PathBuf::from(href);
    }
    base_dir.join(href)
}
