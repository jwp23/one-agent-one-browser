use crate::css::Stylesheet;
use crate::dom::Document;
use crate::url::Url;
use std::sync::Arc;

pub(super) struct UrlLoader {
    pub(super) base_url: Url,
    pub(super) pool: crate::net::FetchPool,
    pub(super) html_request_id: crate::net::RequestId,
    pub(super) html_loaded: bool,
    pub(super) stylesheets: Vec<StylesheetSlot>,
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
            stylesheets: Vec::new(),
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

    pub(super) fn ready_for_screenshot(&self) -> bool {
        if !self.html_loaded {
            return false;
        }
        self.stylesheets.iter().all(|slot| slot.is_loaded())
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
