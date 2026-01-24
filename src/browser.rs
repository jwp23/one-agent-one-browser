use crate::app::TickResult;
use crate::dom::Document;
use crate::render::{DisplayCommand, DisplayList, Painter, Viewport};
use crate::style::StyleComputer;
use crate::url::Url;

pub struct BrowserApp {
    title: String,
    document: Document,
    styles: StyleComputer,
    cached_layout: Option<(Viewport, DisplayList)>,
    url_loader: Option<UrlLoader>,
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
        let title = base_url.as_str().to_owned();
        let loading_document = crate::html::parse_document("<p>Loading...</p>");
        let styles = StyleComputer::from_css("");
        let loader = UrlLoader::new(base_url)?;
        Ok(Self {
            title,
            document: loading_document,
            styles,
            cached_layout: None,
            url_loader: Some(loader),
        })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn tick(&mut self) -> Result<TickResult, String> {
        let Some(mut loader) = self.url_loader.take() else {
            return Ok(TickResult {
                needs_redraw: false,
                ready_for_screenshot: true,
            });
        };

        let mut needs_redraw = false;
        while let Some(event) = loader.pool.try_recv() {
            if event.id == loader.html_request_id && !loader.html_loaded {
                let bytes = event.result.map_err(|err| {
                    format!("Failed to fetch {}: {err}", loader.base_url.as_str())
                })?;
                let html_source = String::from_utf8_lossy(&bytes).into_owned();
                let document = crate::html::parse_document(&html_source);

                loader.stylesheets = loader.fetch_stylesheets(&document)?;
                loader.html_loaded = true;

                self.document = document;
                self.styles = StyleComputer::from_css(&combined_stylesheet_text(&loader.stylesheets));
                self.cached_layout = None;
                needs_redraw = true;
                continue;
            }

            let slot = loader
                .stylesheets
                .iter_mut()
                .find(|slot| slot.request_id() == Some(event.id));
            let Some(slot) = slot else {
                continue;
            };

            let bytes = event
                .result
                .map_err(|err| format!("Failed to fetch {}: {err}", event.url))?;
            let css = String::from_utf8_lossy(&bytes).into_owned();
            slot.set_css(css);
            self.styles = StyleComputer::from_css(&combined_stylesheet_text(&loader.stylesheets));
            self.cached_layout = None;
            needs_redraw = true;
        }

        let ready_for_screenshot = loader.ready_for_screenshot();
        self.url_loader = if ready_for_screenshot { None } else { Some(loader) };

        Ok(TickResult {
            needs_redraw,
            ready_for_screenshot,
        })
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
            url_loader: None,
        })
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

struct UrlLoader {
    base_url: Url,
    pool: crate::net::FetchPool,
    html_request_id: crate::net::RequestId,
    html_loaded: bool,
    stylesheets: Vec<StylesheetSlot>,
}

impl UrlLoader {
    fn new(base_url: Url) -> Result<UrlLoader, String> {
        let mut pool = crate::net::FetchPool::new(8);
        let html_request_id = pool.fetch_bytes(base_url.as_str().to_owned())?;
        Ok(UrlLoader {
            base_url,
            pool,
            html_request_id,
            html_loaded: false,
            stylesheets: Vec::new(),
        })
    }

    fn fetch_stylesheets(&mut self, document: &Document) -> Result<Vec<StylesheetSlot>, String> {
        let mut refs = Vec::new();
        collect_stylesheet_refs(&document.root, &self.base_url, &mut refs)?;

        let mut slots = Vec::with_capacity(refs.len());
        for reference in refs {
            match reference {
                StylesheetRef::Inline(css) => slots.push(StylesheetSlot::Inline(css)),
                StylesheetRef::External(url) => {
                    let id = self.pool.fetch_bytes(url.clone())?;
                    slots.push(StylesheetSlot::External {
                        request_id: id,
                        css: None,
                    });
                }
            }
        }

        Ok(slots)
    }

    fn ready_for_screenshot(&self) -> bool {
        if !self.html_loaded {
            return false;
        }
        self.stylesheets.iter().all(|slot| slot.is_loaded())
    }
}

enum StylesheetSlot {
    Inline(String),
    External {
        request_id: crate::net::RequestId,
        css: Option<String>,
    },
}

impl StylesheetSlot {
    fn request_id(&self) -> Option<crate::net::RequestId> {
        match self {
            StylesheetSlot::Inline(_) => None,
            StylesheetSlot::External { request_id, .. } => Some(*request_id),
        }
    }

    fn set_css(&mut self, css: String) {
        match self {
            StylesheetSlot::Inline(_) => {}
            StylesheetSlot::External { css: slot_css, .. } => {
                *slot_css = Some(css);
            }
        }
    }

    fn is_loaded(&self) -> bool {
        match self {
            StylesheetSlot::Inline(_) => true,
            StylesheetSlot::External { css, .. } => css.is_some(),
        }
    }
}

fn combined_stylesheet_text(slots: &[StylesheetSlot]) -> String {
    let mut out = String::new();
    for slot in slots {
        match slot {
            StylesheetSlot::Inline(css) => {
                out.push_str(css);
                out.push('\n');
            }
            StylesheetSlot::External { css, .. } => {
                if let Some(css) = css {
                    out.push_str(css);
                    out.push('\n');
                }
            }
        }
    }
    out
}

enum StylesheetRef {
    Inline(String),
    External(String),
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
        out.push(StylesheetRef::Inline(css));
    }

    if is_stylesheet_link(element) {
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
                out.push(StylesheetRef::External(url));
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

impl crate::app::App for BrowserApp {
    fn tick(&mut self) -> Result<TickResult, String> {
        BrowserApp::tick(self)
    }

    fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String> {
        BrowserApp::render(self, painter, viewport)
    }
}
