use crate::app::TickResult;
use crate::css::Stylesheet;
use crate::dom::Document;
use crate::render::{DisplayCommand, DisplayList, LinkHitRegion, Painter, Viewport};
use crate::resources::{NoResources, ResourceLoader, ResourceManager};
use crate::style::StyleComputer;
use crate::url::Url;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod render_helpers;

use self::render_helpers::{clip_rect_to_viewport, fill_linear_gradient_rect_clipped};

const STYLES_DEBOUNCE: Duration = Duration::from_millis(80);

pub struct BrowserApp {
    title: String,
    document: Document,
    styles: StyleComputer,
    style_sources: Vec<StylesheetSource>,
    styles_viewport: Option<Viewport>,
    cached_layout: Option<CachedLayout>,
    scroll_y_px: i32,
    url_loader: Option<UrlLoader>,
    base: Option<PageBase>,
    resources: Option<ResourceManager>,
    styles_dirty: bool,
    last_stylesheet_change: Option<Instant>,
}

struct CachedLayout {
    viewport: Viewport,
    display_list: DisplayList,
    link_regions: Vec<LinkHitRegion>,
    document_height_px: i32,
    canvas_background_color: Option<crate::geom::Color>,
}

#[derive(Clone)]
enum PageBase {
    Url(Url),
    FileDir(std::path::PathBuf),
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
        let resource_base = ResourceBase::FileDir(base_dir.clone());
        let mut app = Self::from_html_with_base(&title, &source, Some(resource_base))?;
        app.base = Some(PageBase::FileDir(base_dir.clone()));
        app.resources = Some(ResourceManager::from_file_dir(base_dir));
        Ok(app)
    }

    pub fn from_html(title: &str, html_source: &str) -> Result<Self, String> {
        Self::from_html_with_base(title, html_source, None)
    }

    pub fn from_url(url: &str) -> Result<Self, String> {
        let base_url = Url::parse(url)?;
        let title = base_url.as_str().to_owned();
        let loading_document = crate::html::parse_document("<p>Loading...</p>");
        let styles = StyleComputer::empty();
        let loader = UrlLoader::new(base_url.clone())?;
        Ok(Self {
            title,
            document: loading_document,
            styles,
            style_sources: Vec::new(),
            styles_viewport: None,
            cached_layout: None,
            scroll_y_px: 0,
            url_loader: Some(loader),
            base: Some(PageBase::Url(base_url.clone())),
            resources: Some(ResourceManager::from_url(base_url)),
            styles_dirty: false,
            last_stylesheet_change: None,
        })
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn tick(&mut self) -> Result<TickResult, String> {
        let mut needs_redraw = false;
        let mut ready_for_screenshot = true;
        let mut pending_resources = 0usize;

        if let Some(mut loader) = self.url_loader.take() {
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
                    self.style_sources = stylesheet_sources_from_loader(&loader.stylesheets);
                    self.styles = StyleComputer::empty();
                    self.styles_viewport = None;
                    self.cached_layout = None;
                    self.scroll_y_px = 0;
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
                slot.set_stylesheet(Arc::new(Stylesheet::parse(&css)));
                self.style_sources = stylesheet_sources_from_loader(&loader.stylesheets);
                self.styles = StyleComputer::empty();
                self.styles_viewport = None;
                self.cached_layout = None;
                self.styles_dirty = true;
                self.last_stylesheet_change = Some(Instant::now());
            }

            ready_for_screenshot = loader.ready_for_screenshot();
            self.url_loader = if ready_for_screenshot { None } else { Some(loader) };
        }

        if self.styles_dirty {
            let should_redraw = ready_for_screenshot
                || self
                    .last_stylesheet_change
                    .is_some_and(|instant| instant.elapsed() >= STYLES_DEBOUNCE);
            if should_redraw {
                needs_redraw = true;
            }
        }

        if let Some(resources) = &self.resources {
            let tick = resources.tick();
            if tick.new_successes > 0 {
                self.cached_layout = None;
                needs_redraw = true;
            }
            pending_resources = resources.pending_count();
        }

        if needs_redraw {
            self.styles_dirty = false;
            self.last_stylesheet_change = None;
        }

        Ok(TickResult {
            needs_redraw,
            ready_for_screenshot,
            pending_resources,
        })
    }

    pub fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String> {
        self.ensure_styles_for_viewport(viewport)?;
        if !self
            .cached_layout
            .as_ref()
            .is_some_and(|cached| cached.viewport == viewport)
        {
            let no_resources = NoResources;
            let resources: &dyn ResourceLoader = self
                .resources
                .as_ref()
                .map(|resources| resources as &dyn ResourceLoader)
                .unwrap_or(&no_resources);

            let output =
                crate::layout::layout_document(&self.document, &self.styles, painter, viewport, resources)?;
            self.cached_layout = Some(CachedLayout {
                viewport,
                display_list: output.display_list,
                link_regions: output.link_regions,
                document_height_px: output.document_height_px,
                canvas_background_color: output.canvas_background_color,
            });
        }

        painter.clear()?;

        if let Some(cached) = &self.cached_layout {
            let viewport_width_px = viewport.width_px.max(0);
            let viewport_height_px = viewport.height_px.max(0);

            let max_scroll_y_px = cached
                .document_height_px
                .saturating_sub(viewport_height_px)
                .max(0);
            if self.scroll_y_px > max_scroll_y_px {
                self.scroll_y_px = max_scroll_y_px;
            }
            if self.scroll_y_px < 0 {
                self.scroll_y_px = 0;
            }
            let scroll_y_px = self.scroll_y_px;

            if let Some(color) = cached.canvas_background_color {
                painter.fill_rect(0, 0, viewport_width_px, viewport_height_px, color)?;
            }

            let mut fixed_depth = 0usize;

            for cmd in &cached.display_list.commands {
                match cmd {
                    DisplayCommand::PushFixed => {
                        fixed_depth = fixed_depth.saturating_add(1);
                    }
                    DisplayCommand::PopFixed => {
                        fixed_depth = fixed_depth.saturating_sub(1);
                    }
                    DisplayCommand::PushOpacity(opacity) => painter.push_opacity(*opacity)?,
                    DisplayCommand::PopOpacity(opacity) => painter.pop_opacity(*opacity)?,
                    DisplayCommand::Rect(rect) => {
                        let y_px = if fixed_depth > 0 {
                            rect.y_px
                        } else {
                            rect.y_px.saturating_sub(scroll_y_px)
                        };
                        if let Some((x, y, w, h)) = clip_rect_to_viewport(
                            rect.x_px,
                            y_px,
                            rect.width_px,
                            rect.height_px,
                            viewport_width_px,
                            viewport_height_px,
                        ) {
                            painter.fill_rect(x, y, w, h, rect.color)?;
                        }
                    }
                    DisplayCommand::LinearGradientRect(rect) => {
                        let y_px = if fixed_depth > 0 {
                            rect.y_px
                        } else {
                            rect.y_px.saturating_sub(scroll_y_px)
                        };
                        let translated = crate::render::DrawLinearGradientRect {
                            x_px: rect.x_px,
                            y_px,
                            width_px: rect.width_px,
                            height_px: rect.height_px,
                            direction: rect.direction,
                            start_color: rect.start_color,
                            end_color: rect.end_color,
                        };
                        if let Some((x, y, w, h)) = clip_rect_to_viewport(
                            translated.x_px,
                            translated.y_px,
                            translated.width_px,
                            translated.height_px,
                            viewport_width_px,
                            viewport_height_px,
                        ) {
                            fill_linear_gradient_rect_clipped(painter, &translated, x, y, w, h)?;
                        }
                    }
                    DisplayCommand::RoundedRect(rect) => {
                        let y_px = if fixed_depth > 0 {
                            rect.y_px
                        } else {
                            rect.y_px.saturating_sub(scroll_y_px)
                        };
                        if rect.width_px > 0
                            && rect.height_px > 0
                            && y_px < viewport_height_px
                            && y_px.saturating_add(rect.height_px) > 0
                        {
                            painter.fill_rounded_rect(
                                rect.x_px,
                                y_px,
                                rect.width_px,
                                rect.height_px,
                                rect.radius_px,
                                rect.color,
                            )?;
                        }
                    }
                    DisplayCommand::RoundedRectBorder(rect) => {
                        let y_px = if fixed_depth > 0 {
                            rect.y_px
                        } else {
                            rect.y_px.saturating_sub(scroll_y_px)
                        };
                        if rect.width_px > 0
                            && rect.height_px > 0
                            && y_px < viewport_height_px
                            && y_px.saturating_add(rect.height_px) > 0
                        {
                            painter.stroke_rounded_rect(
                                rect.x_px,
                                y_px,
                                rect.width_px,
                                rect.height_px,
                                rect.radius_px,
                                rect.border_width_px,
                                rect.color,
                            )?;
                        }
                    }
                    DisplayCommand::Text(text) => {
                        let baseline_y_px = if fixed_depth > 0 {
                            text.y_px
                        } else {
                            text.y_px.saturating_sub(scroll_y_px)
                        };
                        let margin_px = text.style.font_size_px.max(0).saturating_mul(4).max(128);
                        let min_baseline_y_px = -margin_px;
                        let max_baseline_y_px = viewport_height_px.saturating_add(margin_px);
                        if baseline_y_px >= min_baseline_y_px && baseline_y_px <= max_baseline_y_px {
                            let metrics = painter.font_metrics_px(text.style);
                            let top = baseline_y_px.saturating_sub(metrics.ascent_px);
                            let bottom = baseline_y_px.saturating_add(metrics.descent_px);
                            if bottom > 0 && top < viewport_height_px {
                                painter.draw_text(text.x_px, baseline_y_px, &text.text, text.style)?;
                            }
                        }
                    }
                    DisplayCommand::Image(image) => {
                        let y_px = if fixed_depth > 0 {
                            image.y_px
                        } else {
                            image.y_px.saturating_sub(scroll_y_px)
                        };
                        if image.width_px > 0
                            && image.height_px > 0
                            && y_px < viewport_height_px
                            && y_px.saturating_add(image.height_px) > 0
                        {
                            painter.draw_image(
                                image.x_px,
                                y_px,
                                image.width_px,
                                image.height_px,
                                image.image.as_ref(),
                                image.opacity,
                            )?;
                        }
                    }
                    DisplayCommand::Svg(svg) => {
                        let y_px = if fixed_depth > 0 {
                            svg.y_px
                        } else {
                            svg.y_px.saturating_sub(scroll_y_px)
                        };
                        if svg.width_px > 0
                            && svg.height_px > 0
                            && y_px < viewport_height_px
                            && y_px.saturating_add(svg.height_px) > 0
                        {
                            painter.draw_svg(
                                svg.x_px,
                                y_px,
                                svg.width_px,
                                svg.height_px,
                                svg.svg_xml.as_ref(),
                                svg.opacity,
                            )?;
                        }
                    }
                }
            }
        }

        painter.flush()?;
        Ok(())
    }

    fn mouse_down(
        &mut self,
        x_px: i32,
        y_px: i32,
        viewport: Viewport,
    ) -> Result<TickResult, String> {
        let Some(cached) = self
            .cached_layout
            .as_ref()
            .filter(|cached| cached.viewport == viewport)
        else {
            return Ok(TickResult::default());
        };

        let Some(href) = cached
            .link_regions
            .iter()
            .rev()
            .find(|region| {
                let hit_y_px = if region.is_fixed {
                    y_px
                } else {
                    y_px.saturating_add(self.scroll_y_px)
                };
                region.contains_point(x_px, hit_y_px)
            })
            .map(|region| region.href.clone())
        else {
            return Ok(TickResult::default());
        };

        self.navigate_href(href.as_ref())?;
        Ok(TickResult {
            needs_redraw: true,
            ready_for_screenshot: false,
            pending_resources: 0,
        })
    }

    fn mouse_wheel(&mut self, delta_y_px: i32, viewport: Viewport) -> Result<TickResult, String> {
        if delta_y_px == 0 {
            return Ok(TickResult {
                needs_redraw: false,
                ready_for_screenshot: true,
                pending_resources: 0,
            });
        }

        let next_unclamped = self.scroll_y_px.saturating_add(delta_y_px).max(0);
        let max_scroll_y_px = self
            .cached_layout
            .as_ref()
            .filter(|cached| cached.viewport == viewport)
            .map(|cached| {
                cached
                    .document_height_px
                    .saturating_sub(viewport.height_px.max(0))
                    .max(0)
            })
            .unwrap_or(i32::MAX);
        let next = next_unclamped.min(max_scroll_y_px);
        let changed = next != self.scroll_y_px;
        self.scroll_y_px = next;
        Ok(TickResult {
            needs_redraw: changed,
            ready_for_screenshot: true,
            pending_resources: 0,
        })
    }
}


impl BrowserApp {
    fn navigate_href(&mut self, href: &str) -> Result<(), String> {
        let href = href.trim();
        if href.is_empty() {
            return Ok(());
        }

        if href.starts_with("http://") || href.starts_with("https://") {
            let Ok(url) = Url::parse(href) else {
                return Ok(());
            };
            return self.begin_url_navigation(url);
        }

        match self.base.clone() {
            Some(PageBase::Url(base)) => {
                let Some(url) = base.resolve(href) else {
                    return Ok(());
                };
                self.begin_url_navigation(url)?;
            }
            Some(PageBase::FileDir(dir)) => {
                let path = resolve_link_file_path(&dir, href);
                if let Err(_) = self.load_file(&path) {
                    return Ok(());
                }
            }
            None => {}
        }

        Ok(())
    }

    fn begin_url_navigation(&mut self, url: Url) -> Result<(), String> {
        self.title = url.as_str().to_owned();
        self.base = Some(PageBase::Url(url.clone()));
        self.resources = Some(ResourceManager::from_url(url.clone()));
        self.document = crate::html::parse_document("<p>Loading...</p>");
        self.styles = StyleComputer::empty();
        self.style_sources = Vec::new();
        self.styles_viewport = None;
        self.cached_layout = None;
        self.url_loader = Some(UrlLoader::new(url)?);
        self.styles_dirty = false;
        self.last_stylesheet_change = None;
        Ok(())
    }

    fn load_file(&mut self, path: &std::path::Path) -> Result<(), String> {
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
        let document = crate::html::parse_document(&source);
        let resource_base = ResourceBase::FileDir(base_dir.clone());
        let style_sources = collect_page_stylesheet_sources(&document, Some(&resource_base))?;

        self.title = title;
        self.document = document;
        self.styles = StyleComputer::empty();
        self.style_sources = style_sources;
        self.styles_viewport = None;
        self.cached_layout = None;
        self.scroll_y_px = 0;
        self.url_loader = None;
        self.base = Some(PageBase::FileDir(base_dir));
        self.resources = match &self.base {
            Some(PageBase::Url(url)) => Some(ResourceManager::from_url(url.clone())),
            Some(PageBase::FileDir(dir)) => Some(ResourceManager::from_file_dir(dir.clone())),
            None => None,
        };
        self.styles_dirty = false;
        self.last_stylesheet_change = None;
        Ok(())
    }

    fn ensure_styles_for_viewport(&mut self, viewport: Viewport) -> Result<(), String> {
        if self.styles_viewport == Some(viewport) {
            return Ok(());
        }

        let mut stylesheets = Vec::new();
        for source in &self.style_sources {
            if let Some(media) = source.media.as_deref() {
                if !crate::css_media::media_query_matches(media, viewport) {
                    continue;
                }
            }
            stylesheets.push(source.stylesheet.clone());
        }

        self.styles = StyleComputer::from_stylesheets(stylesheets);
        self.styles_viewport = Some(viewport);
        self.cached_layout = None;
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
        let style_sources = collect_page_stylesheet_sources(&document, base.as_ref())?;
        let styles = StyleComputer::empty();
        Ok(Self {
            title: title.to_owned(),
            document,
            styles,
            style_sources,
            styles_viewport: None,
            cached_layout: None,
            scroll_y_px: 0,
            url_loader: None,
            base: None,
            resources: None,
            styles_dirty: false,
            last_stylesheet_change: None,
        })
    }
}

#[derive(Clone, Debug)]
struct StylesheetSource {
    stylesheet: Arc<Stylesheet>,
    media: Option<String>,
}

fn collect_page_stylesheet_sources(
    document: &Document,
    base: Option<&ResourceBase>,
) -> Result<Vec<StylesheetSource>, String> {
    let mut out = Vec::new();
    collect_page_stylesheet_sources_from_element(&document.root, base, &mut out)?;
    Ok(out)
}

fn collect_page_stylesheet_sources_from_element(
    element: &crate::dom::Element,
    base: Option<&ResourceBase>,
    out: &mut Vec<StylesheetSource>,
) -> Result<(), String> {
    if element.name == "style" {
        let mut css = String::new();
        for child in &element.children {
            if let crate::dom::Node::Text(text) = child {
                css.push_str(text);
                css.push('\n');
            }
        }
        out.push(StylesheetSource {
            stylesheet: Arc::new(Stylesheet::parse(&css)),
            media: element.attributes.get("media").map(str::to_owned),
        });
    }

    if is_stylesheet_link(element) {
        if let Some(href) = element.attributes.get("href") {
            if let Some(css) = load_stylesheet_text(href, base)? {
                out.push(StylesheetSource {
                    stylesheet: Arc::new(Stylesheet::parse(&css)),
                    media: element.attributes.get("media").map(str::to_owned),
                });
            }
        }
    }

    for child in &element.children {
        if let crate::dom::Node::Element(el) = child {
            collect_page_stylesheet_sources_from_element(el, base, out)?;
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

fn resolve_link_file_path(base_dir: &std::path::Path, href: &str) -> std::path::PathBuf {
    resolve_stylesheet_file_path(base_dir, href)
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

    fn ready_for_screenshot(&self) -> bool {
        if !self.html_loaded {
            return false;
        }
        self.stylesheets.iter().all(|slot| slot.is_loaded())
    }
}

enum StylesheetSlot {
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
    fn request_id(&self) -> Option<crate::net::RequestId> {
        match self {
            StylesheetSlot::Inline { .. } => None,
            StylesheetSlot::External { request_id, .. } => Some(*request_id),
        }
    }

    fn set_stylesheet(&mut self, stylesheet: Arc<Stylesheet>) {
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

fn stylesheet_sources_from_loader(slots: &[StylesheetSlot]) -> Vec<StylesheetSource> {
    let mut out = Vec::new();
    for slot in slots {
        match slot {
            StylesheetSlot::Inline { stylesheet, media } => out.push(StylesheetSource {
                stylesheet: stylesheet.clone(),
                media: media.clone(),
            }),
            StylesheetSlot::External {
                stylesheet: Some(stylesheet),
                media,
                ..
            } => out.push(StylesheetSource {
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
    Inline {
        css: String,
        media: Option<String>,
    },
    External {
        url: String,
        media: Option<String>,
    },
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

impl crate::app::App for BrowserApp {
    fn tick(&mut self) -> Result<TickResult, String> {
        BrowserApp::tick(self)
    }

    fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String> {
        BrowserApp::render(self, painter, viewport)
    }

    fn mouse_down(&mut self, x_px: i32, y_px: i32, viewport: Viewport) -> Result<TickResult, String> {
        BrowserApp::mouse_down(self, x_px, y_px, viewport)
    }

    fn mouse_wheel(&mut self, delta_y_px: i32, viewport: Viewport) -> Result<TickResult, String> {
        BrowserApp::mouse_wheel(self, delta_y_px, viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stylesheets_are_parsed_once_and_reused_across_viewports() {
        crate::css::reset_stylesheet_parse_call_count();
        let html = "<style>body { margin: 0; }</style><style>p { color: #123456; }</style><p>t</p>";

        let mut app = BrowserApp::from_html("test", html).unwrap();
        let parsed = crate::css::stylesheet_parse_call_count();
        assert_eq!(parsed, 2);

        app.ensure_styles_for_viewport(Viewport {
            width_px: 320,
            height_px: 200,
        })
        .unwrap();
        app.ensure_styles_for_viewport(Viewport {
            width_px: 480,
            height_px: 200,
        })
        .unwrap();

        assert_eq!(crate::css::stylesheet_parse_call_count(), parsed);
    }
}
