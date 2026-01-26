use crate::dom::Element;
use crate::geom::{Color, Rect};
use crate::render::{DisplayCommand, DrawImage, DrawSvg, DrawText};
use crate::style::ComputedStyle;
use std::rc::Rc;

use super::{inline, LayoutEngine};

impl LayoutEngine<'_> {
    pub(super) fn paint_replaced_content(
        &mut self,
        element: &Element,
        style: &ComputedStyle,
        content_box: Rect,
    ) -> Result<(), String> {
        if content_box.width <= 0 || content_box.height <= 0 {
            return Ok(());
        }

        match element.name.as_str() {
            "img" => {
                if let Some(src) = element.attributes.get("src") {
                    if let Some(image) = self.load_image(src)? {
                        self.list.commands.push(DisplayCommand::Image(DrawImage {
                            x_px: content_box.x,
                            y_px: content_box.y,
                            width_px: content_box.width,
                            height_px: content_box.height,
                            opacity: 255,
                            image,
                        }));
                    }
                }
            }
            "svg" => {
                let xml = inline::serialize_element_xml(element);
                self.list.commands.push(DisplayCommand::Svg(DrawSvg {
                    x_px: content_box.x,
                    y_px: content_box.y,
                    width_px: content_box.width,
                    height_px: content_box.height,
                    opacity: 255,
                    svg_xml: Rc::from(xml),
                }));
            }
            "input" => self.paint_input_control(element, style, content_box)?,
            _ => {}
        }

        Ok(())
    }

    fn paint_input_control(
        &mut self,
        element: &Element,
        style: &ComputedStyle,
        content_box: Rect,
    ) -> Result<(), String> {
        let input_type = element
            .attributes
            .get("type")
            .unwrap_or("text")
            .trim()
            .to_ascii_lowercase();

        let (text, center_text, is_placeholder) = match input_type.as_str() {
            "submit" | "button" | "reset" => {
                let mut label = element.attributes.get("value").unwrap_or("").trim();
                if label.is_empty() {
                    label = match input_type.as_str() {
                        "reset" => "Reset",
                        _ => "Submit",
                    };
                }
                (label, true, false)
            }
            _ => {
                let value = element.attributes.get("value").unwrap_or("").trim();
                if !value.is_empty() {
                    (value, false, false)
                } else {
                    let placeholder = element.attributes.get("placeholder").unwrap_or("").trim();
                    (placeholder, false, true)
                }
            }
        };

        if text.is_empty() {
            return Ok(());
        }

        let transformed = style.text_transform.apply(text);
        let text = transformed.as_ref();

        let mut text_style = self.text_style_for(style);
        text_style.underline = false;
        if is_placeholder {
            text_style.color = placeholder_color(text_style.color);
        }

        let metrics = self.measurer.font_metrics_px(text_style);
        let ascent_px = metrics.ascent_px.max(1);
        let descent_px = metrics.descent_px.max(0);
        let text_height_px = ascent_px.saturating_add(descent_px).max(1);
        let y_offset = content_box
            .height
            .saturating_sub(text_height_px)
            .max(0)
            / 2;
        let baseline_y = content_box.y.saturating_add(y_offset).saturating_add(ascent_px);

        let mut x_px = content_box.x;
        if center_text {
            let text_width_px = self.measurer.text_width_px(text, text_style)?;
            x_px = x_px.saturating_add(
                content_box
                    .width
                    .saturating_sub(text_width_px.max(0))
                    .max(0)
                    / 2,
            );
        }

        self.list.commands.push(DisplayCommand::Text(DrawText {
            x_px,
            y_px: baseline_y,
            text: text.to_owned(),
            style: text_style,
        }));

        Ok(())
    }
}

fn placeholder_color(base: Color) -> Color {
    fn mix_channel(channel: u8) -> u8 {
        ((channel as u16 + 255) / 2) as u8
    }

    Color {
        r: mix_channel(base.r),
        g: mix_channel(base.g),
        b: mix_channel(base.b),
        a: base.a,
    }
}
