use crate::dom::Document;
use crate::render::{DisplayCommand, DisplayList, Painter, Viewport};
use crate::style::StyleComputer;

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
        Self::from_html(&title, &source)
    }

    pub fn from_html(title: &str, html_source: &str) -> Result<Self, String> {
        let document = crate::html::parse_document(html_source);
        let styles = StyleComputer::from_document(&document);
        Ok(Self {
            title: title.to_owned(),
            document,
            styles,
            cached_layout: None,
        })
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
