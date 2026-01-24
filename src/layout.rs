use crate::dom::{Document, Element, Node};
use crate::render::{DisplayList, DrawText, TextMeasurer, TextStyle, Viewport};

const PADDING_X_PX: i32 = 24;
const PADDING_Y_PX: i32 = 24;
const PARAGRAPH_SPACING_LINES: i32 = 1;

pub fn layout_document(
    document: &Document,
    measurer: &dyn TextMeasurer,
    viewport: Viewport,
) -> Result<DisplayList, String> {
    let mut tokens = Vec::new();
    let mut cursor = FlowCursor::default();
    let root = document.render_root();
    collect_flow_from_element(root, TextStyle::default(), &mut cursor, &mut tokens);

    let max_width_px = viewport
        .width_px
        .saturating_sub(PADDING_X_PX.saturating_mul(2))
        .max(0);

    let line_height_px = measurer.line_height_px().max(1);
    let mut x_px: i32 = 0;
    let mut line_index: i32 = 0;
    let mut display = DisplayList::default();

    let mut line = LineBuilder::default();

    for token in tokens {
        match token {
            FlowToken::Newline => {
                flush_line(&mut display, &mut line);
                x_px = 0;
                line_index = line_index.saturating_add(1);
                if baseline_y_px(line_index, line_height_px) > viewport.height_px {
                    break;
                }
            }
            FlowToken::Word { text, style } => {
                if text.is_empty() {
                    continue;
                }
                let word_width_px = measurer.text_width_px(&text)?;

                if x_px != 0 && x_px.saturating_add(word_width_px) > max_width_px {
                    flush_line(&mut display, &mut line);
                    x_px = 0;
                    line_index = line_index.saturating_add(1);
                    if baseline_y_px(line_index, line_height_px) > viewport.height_px {
                        break;
                    }
                }

                if x_px == 0 && word_width_px > max_width_px && max_width_px > 0 {
                    let mut remaining = text.as_str();
                    while !remaining.is_empty() {
                        let fit = fit_prefix_bytes(measurer, remaining, max_width_px)?;
                        if fit == 0 {
                            break;
                        }
                        let (prefix, rest) = remaining.split_at(fit);
                        line.push(
                            DrawText {
                                x_px: PADDING_X_PX + x_px,
                                y_px: baseline_y_px(line_index, line_height_px),
                                text: prefix.to_owned(),
                                style,
                            },
                        );
                        x_px = x_px.saturating_add(measurer.text_width_px(prefix)?);
                        if !rest.is_empty() {
                            flush_line(&mut display, &mut line);
                            x_px = 0;
                            line_index = line_index.saturating_add(1);
                            if baseline_y_px(line_index, line_height_px) > viewport.height_px {
                                break;
                            }
                        }
                        remaining = rest;
                    }
                    continue;
                }

                line.push(
                    DrawText {
                        x_px: PADDING_X_PX + x_px,
                        y_px: baseline_y_px(line_index, line_height_px),
                        text,
                        style,
                    },
                );
                x_px = x_px.saturating_add(word_width_px);
            }
            FlowToken::Space => {
                if x_px == 0 {
                    continue;
                }
                let space_width_px = measurer.text_width_px(" ")?;
                if x_px.saturating_add(space_width_px) > max_width_px {
                    continue;
                }
                line.push(
                    DrawText {
                        x_px: PADDING_X_PX + x_px,
                        y_px: baseline_y_px(line_index, line_height_px),
                        text: " ".to_owned(),
                        style: TextStyle::default(),
                    },
                );
                x_px = x_px.saturating_add(space_width_px);
            }
        }
    }

    flush_line(&mut display, &mut line);

    Ok(display)
}

#[derive(Default)]
struct FlowCursor {
    pending_space: bool,
}

enum FlowToken {
    Word { text: String, style: TextStyle },
    Space,
    Newline,
}

fn collect_flow_from_element(
    element: &Element,
    style: TextStyle,
    cursor: &mut FlowCursor,
    out: &mut Vec<FlowToken>,
) {
    match element.name.as_str() {
        "p" => {
            push_newline_if_needed(out);
            cursor.pending_space = false;
            collect_children(&element.children, style, cursor, out);
            out.push(FlowToken::Newline);
            for _ in 0..PARAGRAPH_SPACING_LINES {
                out.push(FlowToken::Newline);
            }
            cursor.pending_space = false;
        }
        "br" => {
            out.push(FlowToken::Newline);
            cursor.pending_space = false;
        }
        "strong" => {
            let mut bold_style = style;
            bold_style.bold = true;
            collect_children(&element.children, bold_style, cursor, out);
        }
        "script" | "style" | "head" => {}
        _ => collect_children(&element.children, style, cursor, out),
    }
}

fn collect_children(children: &[Node], style: TextStyle, cursor: &mut FlowCursor, out: &mut Vec<FlowToken>) {
    for child in children {
        match child {
            Node::Text(text) => push_text(text, style, cursor, out),
            Node::Element(el) => collect_flow_from_element(el, style, cursor, out),
        }
    }
}

fn push_text(text: &str, style: TextStyle, cursor: &mut FlowCursor, out: &mut Vec<FlowToken>) {
    for word in text.split_whitespace() {
        if cursor.pending_space {
            if matches!(out.last(), Some(FlowToken::Newline) | None) {
                // Avoid spaces at the start of a line/paragraph.
            } else {
                out.push(FlowToken::Space);
            }
        }
        out.push(FlowToken::Word {
            text: word.to_owned(),
            style,
        });
        cursor.pending_space = true;
    }
}

fn push_newline_if_needed(out: &mut Vec<FlowToken>) {
    if matches!(out.last(), Some(FlowToken::Newline) | None) {
        return;
    }
    out.push(FlowToken::Newline);
}

#[derive(Default)]
struct LineBuilder {
    texts: Vec<DrawText>,
}

impl LineBuilder {
    fn push(&mut self, draw: DrawText) {
        self.texts.push(draw);
    }

    fn take_texts(&mut self) -> Vec<DrawText> {
        std::mem::take(&mut self.texts)
    }
}

fn flush_line(display: &mut DisplayList, line: &mut LineBuilder) {
    display.texts.extend(line.take_texts());
}

fn baseline_y_px(line_index: i32, line_height_px: i32) -> i32 {
    PADDING_Y_PX + line_height_px.saturating_mul(line_index.saturating_add(1))
}

fn fit_prefix_bytes(
    measurer: &dyn TextMeasurer,
    text: &str,
    max_width_px: i32,
) -> Result<usize, String> {
    let bytes = text.as_bytes();
    let mut low = 0usize;
    let mut high = bytes.len();

    while low < high {
        let mid = (low + high + 1) / 2;
        if !text.is_char_boundary(mid) {
            high = mid - 1;
            continue;
        }
        let width = measurer.text_width_px(&text[..mid])?;
        if width <= max_width_px {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    Ok(low)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn line_height_px(&self) -> i32 {
            10
        }

        fn text_width_px(&self, text: &str) -> Result<i32, String> {
            Ok(text.len() as i32)
        }
    }

    #[test]
    fn wraps_words_when_exceeding_width() {
        let doc = crate::html::parse_document("<p>Hello World</p>");
        let viewport = Viewport {
            width_px: PADDING_X_PX * 2 + 5,
            height_px: 200,
        };
        let list = layout_document(&doc, &FixedMeasurer, viewport).unwrap();
        assert!(list.texts.len() >= 2);
    }
}
