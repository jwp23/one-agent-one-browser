use super::d2d;
use super::d3d11;
use super::dwrite;
use super::gdi;
use super::svg;
use super::wstr;
use crate::debug;
use crate::geom::Color;
use crate::image::{Argb32Image, RgbImage};
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use crate::style::FontFamily;
use crate::win::stream;
use crate::win::com::ComPtr;
use core::ffi::c_void;
use std::collections::HashMap;

type HWND = *mut c_void;

const MAX_TEXT_LAYOUT_EXTENT_PX: f32 = 1_000_000.0;

pub(super) struct WinPainter {
    hwnd: Option<HWND>,
    width_px: i32,
    height_px: i32,
    bgra: Vec<u8>,
    in_draw: bool,
    opacity_layers: Vec<ComPtr<d2d::ID2D1Layer>>,
    brush_cache: HashMap<u32, ComPtr<d2d::ID2D1SolidColorBrush>>,
    text_formats: std::cell::RefCell<HashMap<FontKey, ComPtr<dwrite::IDWriteTextFormat>>>,
    font_metrics: std::cell::RefCell<HashMap<FontKey, FontMetricsPx>>,

    _d3d: d3d11::D3DDevices,
    _d2d_factory: ComPtr<d2d::ID2D1Factory1>,
    _d2d_device: ComPtr<d2d::ID2D1Device>,
    d2d_ctx: ComPtr<d2d::ID2D1DeviceContext5>,
    d2d_target: ComPtr<d2d::ID2D1Bitmap1>,
    d2d_readback: ComPtr<d2d::ID2D1Bitmap1>,
    dwrite_factory: ComPtr<dwrite::IDWriteFactory>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct FontKey {
    family: FontFamily,
    size_px: i32,
    bold: bool,
}

impl WinPainter {
    pub(super) fn new(viewport: Viewport, hwnd: Option<HWND>) -> Result<Self, String> {
        let (width_px, height_px) = validate_viewport(viewport)?;

        let d3d = d3d11::create_d3d_devices()?;
        let d2d_factory = d2d::create_factory1()?;
        let d2d_device = d2d::factory_create_device(
            &d2d_factory,
            d3d.dxgi_device.as_ptr().cast::<c_void>(),
        )
        .map_err(|err| err.message())?;
        let d2d_ctx = d2d::device_create_device_context(&d2d_device).map_err(|err| err.message())?;

        d2d::ctx_set_unit_mode(&d2d_ctx, d2d::D2D1_UNIT_MODE_PIXELS);
        d2d::ctx_set_text_antialias_mode(&d2d_ctx, d2d::D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
        d2d::ctx_set_transform(&d2d_ctx, &d2d::D2D1_IDENTITY_MATRIX);

        let dwrite_factory = dwrite::create_factory()?;

        let (d2d_target, d2d_readback, bgra) = create_back_buffers(&d2d_ctx, width_px, height_px)?;
        d2d::ctx_set_target(&d2d_ctx, &d2d_target);

        Ok(Self {
            hwnd,
            width_px,
            height_px,
            bgra,
            in_draw: false,
            opacity_layers: Vec::new(),
            brush_cache: HashMap::new(),
            text_formats: std::cell::RefCell::new(HashMap::new()),
            font_metrics: std::cell::RefCell::new(HashMap::new()),
            _d3d: d3d,
            _d2d_factory: d2d_factory,
            _d2d_device: d2d_device,
            d2d_ctx,
            d2d_target,
            d2d_readback,
            dwrite_factory,
        })
    }

    pub(super) fn ensure_back_buffer(&mut self, viewport: Viewport) -> Result<(), String> {
        let (width_px, height_px) = validate_viewport(viewport)?;
        if width_px == self.width_px && height_px == self.height_px {
            return Ok(());
        }

        if self.in_draw {
            let _ = d2d::ctx_end_draw(&self.d2d_ctx);
            self.in_draw = false;
        }

        if !self.opacity_layers.is_empty() {
            debug::log(
                debug::Target::Render,
                debug::Level::Warn,
                format_args!(
                    "Windows painter: opacity stack was not empty during resize (depth={})",
                    self.opacity_layers.len()
                ),
            );
            while self.opacity_layers.pop().is_some() {
                d2d::ctx_pop_layer(&self.d2d_ctx);
            }
        }

        let (target, readback, bgra) = create_back_buffers(&self.d2d_ctx, width_px, height_px)?;
        self.d2d_target = target;
        self.d2d_readback = readback;
        self.bgra = bgra;
        self.width_px = width_px;
        self.height_px = height_px;

        d2d::ctx_set_target(&self.d2d_ctx, &self.d2d_target);
        d2d::ctx_set_transform(&self.d2d_ctx, &d2d::D2D1_IDENTITY_MATRIX);
        self.brush_cache.clear();

        Ok(())
    }

    pub(super) fn capture_back_buffer_rgb(&self) -> Result<RgbImage, String> {
        let width_u32: u32 = self
            .width_px
            .try_into()
            .map_err(|_| "Screenshot width out of range".to_owned())?;
        let height_u32: u32 = self
            .height_px
            .try_into()
            .map_err(|_| "Screenshot height out of range".to_owned())?;

        let expected_len = (width_u32 as usize)
            .checked_mul(height_u32 as usize)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or_else(|| "Screenshot buffer size overflow".to_owned())?;
        let mut rgb = Vec::with_capacity(expected_len);

        for chunk in self.bgra.chunks_exact(4) {
            rgb.push(chunk[2]);
            rgb.push(chunk[1]);
            rgb.push(chunk[0]);
        }

        RgbImage::new(width_u32, height_u32, rgb)
    }

    fn begin_draw_if_needed(&mut self) {
        if self.in_draw {
            return;
        }
        d2d::ctx_begin_draw(&self.d2d_ctx);
        d2d::ctx_set_unit_mode(&self.d2d_ctx, d2d::D2D1_UNIT_MODE_PIXELS);
        d2d::ctx_set_text_antialias_mode(&self.d2d_ctx, d2d::D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
        d2d::ctx_set_transform(&self.d2d_ctx, &d2d::D2D1_IDENTITY_MATRIX);
        self.in_draw = true;
    }

    fn brush_for(&mut self, color: Color) -> Result<*mut d2d::ID2D1SolidColorBrush, String> {
        let key = (u32::from(color.r) << 24)
            | (u32::from(color.g) << 16)
            | (u32::from(color.b) << 8)
            | u32::from(color.a);
        if !self.brush_cache.contains_key(&key) {
            let brush = d2d::ctx_create_solid_color_brush(&self.d2d_ctx, &to_d2d_color(color))
                .map_err(|err| err.message())?;
            self.brush_cache.insert(key, brush);
        }
        self.brush_cache
            .get(&key)
            .map(|brush| brush.as_ptr())
            .ok_or_else(|| "Internal error: brush cache missing entry".to_owned())
    }

    fn text_format_ptr(&self, style: TextStyle) -> Result<*mut dwrite::IDWriteTextFormat, String> {
        let key = FontKey {
            family: style.font_family,
            size_px: style.font_size_px.max(1),
            bold: style.bold,
        };

        let mut cache = self.text_formats.borrow_mut();
        if !cache.contains_key(&key) {
            let family_name = match key.family {
                FontFamily::SansSerif => "Segoe UI",
                FontFamily::Serif => "Times New Roman",
                FontFamily::Monospace => "Consolas",
            };
            let family_w = wstr::utf16_nul(family_name);
            let locale_w = wstr::utf16_nul("en-us");
            let weight = if key.bold {
                dwrite::DWRITE_FONT_WEIGHT_BOLD
            } else {
                dwrite::DWRITE_FONT_WEIGHT_NORMAL
            };

            let format = dwrite::create_text_format(
                &self.dwrite_factory,
                family_w.as_ptr(),
                locale_w.as_ptr(),
                weight,
                dwrite::DWRITE_FONT_STYLE_NORMAL,
                dwrite::DWRITE_FONT_STRETCH_NORMAL,
                key.size_px as f32,
            )
            .map_err(|err| err.message())?;
            cache.insert(key, format);
        }

        cache
            .get(&key)
            .map(|fmt| fmt.as_ptr())
            .ok_or_else(|| "Internal error: text format cache missing entry".to_owned())
    }

    fn text_width_no_spacing(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        if text.is_empty() {
            return Ok(0);
        }

        let format_ptr = self.text_format_ptr(style)?;
        let text_w: Vec<u16> = text.encode_utf16().collect();
        let len: u32 = text_w
            .len()
            .try_into()
            .map_err(|_| "Text is too large".to_owned())?;
        let layout = dwrite::create_text_layout(
            &self.dwrite_factory,
            text_w.as_ptr(),
            len,
            format_ptr,
            MAX_TEXT_LAYOUT_EXTENT_PX,
            MAX_TEXT_LAYOUT_EXTENT_PX,
        )
        .map_err(|err| err.message())?;

        let metrics = dwrite::text_layout_get_metrics(&layout).map_err(|err| err.message())?;
        let w = metrics.width_including_trailing_whitespace;
        if !w.is_finite() || w <= 0.0 {
            return Ok(0);
        }

        let w = w.round() as i64;
        Ok(w.clamp(0, i64::from(i32::MAX)) as i32)
    }

    fn draw_text_run(&mut self, x_px: i32, baseline_y_px: i32, text: &str, style: TextStyle) -> Result<(), String> {
        if text.is_empty() || style.color.a == 0 {
            return Ok(());
        }

        let metrics = self.font_metrics_px(style);
        let origin = d2d::D2D1_POINT_2F {
            x: x_px as f32,
            y: baseline_y_px.saturating_sub(metrics.ascent_px) as f32,
        };

        let format_ptr = self.text_format_ptr(style)?;
        let text_w: Vec<u16> = text.encode_utf16().collect();
        let len: u32 = text_w
            .len()
            .try_into()
            .map_err(|_| "Text is too large".to_owned())?;
        let layout = dwrite::create_text_layout(
            &self.dwrite_factory,
            text_w.as_ptr(),
            len,
            format_ptr,
            MAX_TEXT_LAYOUT_EXTENT_PX,
            MAX_TEXT_LAYOUT_EXTENT_PX,
        )
        .map_err(|err| err.message())?;

        self.begin_draw_if_needed();
        let brush = self.brush_for(style.color)?;
        d2d::ctx_draw_text_layout(
            &self.d2d_ctx,
            origin,
            layout.as_ptr().cast::<c_void>(),
            brush,
            d2d::D2D1_DRAW_TEXT_OPTIONS_NONE,
            dwrite::DWRITE_MEASURING_MODE_NATURAL,
        );
        Ok(())
    }
}

impl TextMeasurer for WinPainter {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        let key = FontKey {
            family: style.font_family,
            size_px: style.font_size_px.max(1),
            bold: style.bold,
        };

        if let Some(metrics) = self.font_metrics.borrow().get(&key) {
            return *metrics;
        }

        let computed = (|| -> Result<FontMetricsPx, String> {
            let format_ptr = self.text_format_ptr(style)?;
            let sample = "Hg";
            let sample_w: Vec<u16> = sample.encode_utf16().collect();
            let len: u32 = sample_w
                .len()
                .try_into()
                .map_err(|_| "Text is too large".to_owned())?;
            let layout = dwrite::create_text_layout(
                &self.dwrite_factory,
                sample_w.as_ptr(),
                len,
                format_ptr,
                MAX_TEXT_LAYOUT_EXTENT_PX,
                MAX_TEXT_LAYOUT_EXTENT_PX,
            )
            .map_err(|err| err.message())?;

            let lines = dwrite::text_layout_get_line_metrics(&layout).map_err(|err| err.message())?;
            let Some(line0) = lines.first() else {
                return Ok(FontMetricsPx {
                    ascent_px: style.font_size_px.max(1),
                    descent_px: 0,
                });
            };
            if !line0.height.is_finite() || !line0.baseline.is_finite() {
                return Ok(FontMetricsPx {
                    ascent_px: style.font_size_px.max(1),
                    descent_px: 0,
                });
            }

            let ascent = line0.baseline.round() as i32;
            let descent = (line0.height - line0.baseline).round() as i32;
            Ok(FontMetricsPx {
                ascent_px: ascent.max(1),
                descent_px: descent.max(0),
            })
        })()
        .unwrap_or(FontMetricsPx {
            ascent_px: style.font_size_px.max(1),
            descent_px: 0,
        });

        self.font_metrics.borrow_mut().insert(key, computed);
        computed
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        if style.letter_spacing_px == 0 {
            return self.text_width_no_spacing(text, style);
        }

        let mut total_width: i64 = 0;
        let mut first = true;
        for ch in text.chars() {
            if !first {
                total_width += i64::from(style.letter_spacing_px);
            }
            first = false;

            let mut buf = [0u8; 4];
            let ch = ch.encode_utf8(&mut buf);
            total_width += i64::from(self.text_width_no_spacing(ch, style)?);
        }

        Ok(total_width.clamp(0, i64::from(i32::MAX)) as i32)
    }
}

impl Painter for WinPainter {
    fn clear(&mut self) -> Result<(), String> {
        self.begin_draw_if_needed();
        d2d::ctx_clear(
            &self.d2d_ctx,
            &d2d::D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        );
        Ok(())
    }

    fn push_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if opacity >= 255 {
            return Ok(());
        }
        self.begin_draw_if_needed();

        let layer = d2d::ctx_create_layer(&self.d2d_ctx).map_err(|err| err.message())?;
        let params = d2d::D2D1_LAYER_PARAMETERS1 {
            content_bounds: d2d::D2D1_RECT_F {
                left: 0.0,
                top: 0.0,
                right: self.width_px.max(0) as f32,
                bottom: self.height_px.max(0) as f32,
            },
            geometric_mask: std::ptr::null_mut(),
            mask_antialias_mode: d2d::D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            mask_transform: d2d::D2D1_IDENTITY_MATRIX,
            opacity: (opacity as f32) / 255.0,
            opacity_brush: std::ptr::null_mut(),
            layer_options1: d2d::D2D1_LAYER_OPTIONS1_NONE,
        };

        d2d::ctx_push_layer(&self.d2d_ctx, &params, &layer);
        self.opacity_layers.push(layer);
        Ok(())
    }

    fn pop_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if opacity >= 255 {
            return Ok(());
        }
        if self.opacity_layers.is_empty() {
            return Err("opacity stack underflow".to_owned());
        }

        self.begin_draw_if_needed();
        d2d::ctx_pop_layer(&self.d2d_ctx);
        let _ = self.opacity_layers.pop();
        Ok(())
    }

    fn fill_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || color.a == 0 {
            return Ok(());
        }

        self.begin_draw_if_needed();
        let rect = d2d::D2D1_RECT_F {
            left: x_px as f32,
            top: y_px as f32,
            right: x_px.saturating_add(width_px) as f32,
            bottom: y_px.saturating_add(height_px) as f32,
        };
        let brush = self.brush_for(color)?;
        d2d::ctx_fill_rectangle(&self.d2d_ctx, &rect, brush);
        Ok(())
    }

    fn fill_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || color.a == 0 {
            return Ok(());
        }

        self.begin_draw_if_needed();
        let radius = radius_px.max(0) as f32;
        let rect = d2d::D2D1_ROUNDED_RECT {
            rect: d2d::D2D1_RECT_F {
                left: x_px as f32,
                top: y_px as f32,
                right: x_px.saturating_add(width_px) as f32,
                bottom: y_px.saturating_add(height_px) as f32,
            },
            radius_x: radius,
            radius_y: radius,
        };
        let brush = self.brush_for(color)?;
        d2d::ctx_fill_rounded_rectangle(&self.d2d_ctx, &rect, brush);
        Ok(())
    }

    fn stroke_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        border_width_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || color.a == 0 {
            return Ok(());
        }
        if border_width_px <= 0 {
            return Ok(());
        }

        self.begin_draw_if_needed();
        let radius = radius_px.max(0) as f32;
        let rect = d2d::D2D1_ROUNDED_RECT {
            rect: d2d::D2D1_RECT_F {
                left: x_px as f32,
                top: y_px as f32,
                right: x_px.saturating_add(width_px) as f32,
                bottom: y_px.saturating_add(height_px) as f32,
            },
            radius_x: radius,
            radius_y: radius,
        };
        let brush = self.brush_for(color)?;
        d2d::ctx_draw_rounded_rectangle(&self.d2d_ctx, &rect, brush, border_width_px as f32);
        Ok(())
    }

    fn draw_text(&mut self, x_px: i32, y_px: i32, text: &str, style: TextStyle) -> Result<(), String> {
        if text.is_empty() || style.color.a == 0 {
            return Ok(());
        }

        if style.letter_spacing_px == 0 {
            self.draw_text_run(x_px, y_px, text, style)?;
        } else {
            let mut cursor_x = x_px;
            let mut first = true;
            for ch in text.chars() {
                if !first {
                    cursor_x = cursor_x.saturating_add(style.letter_spacing_px);
                }
                first = false;

                let mut buf = [0u8; 4];
                let ch = ch.encode_utf8(&mut buf);
                self.draw_text_run(cursor_x, y_px, ch, style)?;
                cursor_x = cursor_x.saturating_add(self.text_width_no_spacing(ch, style)?);
            }
        }

        if style.underline {
            let width_px = self.text_width_px(text, style)?;
            self.fill_rect(x_px, y_px.saturating_add(1), width_px, 1, style.color)?;
        }

        Ok(())
    }

    fn draw_image(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        image: &Argb32Image,
        opacity: u8,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || opacity == 0 {
            return Ok(());
        }
        if image.width == 0 || image.height == 0 {
            return Ok(());
        }

        let stride: u32 = image
            .row_stride_bytes()
            .try_into()
            .map_err(|_| "Image row stride out of range".to_owned())?;
        let bitmap = d2d::ctx_create_bitmap(
            &self.d2d_ctx,
            d2d::D2D1_SIZE_U {
                width: image.width,
                height: image.height,
            },
            Some((image.data.as_ptr(), stride)),
            0,
        )
        .map_err(|err| err.message())?;

        self.begin_draw_if_needed();
        let rect = d2d::D2D1_RECT_F {
            left: x_px as f32,
            top: y_px as f32,
            right: x_px.saturating_add(width_px) as f32,
            bottom: y_px.saturating_add(height_px) as f32,
        };
        d2d::ctx_draw_bitmap(&self.d2d_ctx, &bitmap, &rect, (opacity as f32) / 255.0);
        Ok(())
    }

    fn draw_svg(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        svg_xml: &str,
        opacity: u8,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || opacity == 0 {
            return Ok(());
        }

        let svg_xml = svg_xml.trim();
        if svg_xml.is_empty() {
            return Ok(());
        }

        self.begin_draw_if_needed();

        let svg_xml = svg::ensure_xlink_namespace(svg_xml);
        let stream = stream::create_istream_from_bytes(svg_xml.as_ref().as_bytes())
            .map_err(|err| err.message())?;

        let doc = match d2d::ctx_create_svg_document(
            &self.d2d_ctx,
            stream.as_ptr().cast::<c_void>(),
            d2d::D2D1_SIZE_F {
                width: width_px as f32,
                height: height_px as f32,
            },
        ) {
            Ok(doc) => doc,
            Err(err) => {
                debug::log(
                    debug::Target::Render,
                    debug::Level::Warn,
                    format_args!(
                        "SVG render failed: {}",
                        debug::shorten(&err.message(), 120)
                    ),
                );
                return Ok(());
            }
        };

        let needs_local_opacity = opacity < 255;
        if needs_local_opacity {
            self.push_opacity(opacity)?;
        }

        let translate = d2d::D2D1_MATRIX_3X2_F {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            dx: x_px as f32,
            dy: y_px as f32,
        };
        d2d::ctx_set_transform(&self.d2d_ctx, &translate);
        d2d::ctx_draw_svg_document(&self.d2d_ctx, &doc);
        d2d::ctx_set_transform(&self.d2d_ctx, &d2d::D2D1_IDENTITY_MATRIX);

        if needs_local_opacity {
            self.pop_opacity(opacity)?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        if self.in_draw {
            d2d::ctx_end_draw(&self.d2d_ctx).map_err(|err| err.message())?;
            self.in_draw = false;
        }

        if !self.opacity_layers.is_empty() {
            debug::log(
                debug::Target::Render,
                debug::Level::Warn,
                format_args!(
                    "Windows painter: opacity stack was not empty after flush (depth={})",
                    self.opacity_layers.len()
                ),
            );
            while self.opacity_layers.pop().is_some() {
                d2d::ctx_pop_layer(&self.d2d_ctx);
            }
        }

        d2d::bitmap_copy_from_bitmap(&self.d2d_readback, &self.d2d_target).map_err(|err| err.message())?;
        let mapped = d2d::bitmap_map(&self.d2d_readback, d2d::D2D1_MAP_OPTIONS_READ).map_err(|err| err.message())?;
        if mapped.bits.is_null() {
            let _ = d2d::bitmap_unmap(&self.d2d_readback);
            return Err("ID2D1Bitmap1::Map returned null bits".to_owned());
        }

        let row_bytes = (self.width_px as usize)
            .checked_mul(4)
            .ok_or_else(|| "Back buffer row size overflow".to_owned())?;
        let height_usize: usize = self
            .height_px
            .try_into()
            .map_err(|_| "Back buffer height out of range".to_owned())?;
        let expected_len = row_bytes
            .checked_mul(height_usize)
            .ok_or_else(|| "Back buffer size overflow".to_owned())?;
        if self.bgra.len() != expected_len {
            self.bgra.resize(expected_len, 0);
        }

        let src_pitch: usize = mapped.pitch as usize;
        for row in 0..height_usize {
            let src = unsafe {
                std::slice::from_raw_parts(
                    mapped.bits.add(row.saturating_mul(src_pitch)),
                    src_pitch,
                )
            };
            let dst_offset = row.saturating_mul(row_bytes);
            let dst = self
                .bgra
                .get_mut(dst_offset..dst_offset + row_bytes)
                .ok_or_else(|| "Back buffer write out of bounds".to_owned())?;
            dst.copy_from_slice(&src[..row_bytes]);
        }

        d2d::bitmap_unmap(&self.d2d_readback).map_err(|err| err.message())?;

        if let Some(hwnd) = self.hwnd {
            gdi::blit_bgra(hwnd, self.width_px, self.height_px, &self.bgra)?;
        }

        Ok(())
    }
}

fn validate_viewport(viewport: Viewport) -> Result<(i32, i32), String> {
    let width_px = viewport.width_px;
    let height_px = viewport.height_px;
    if width_px <= 0 || height_px <= 0 {
        return Err(format!("Invalid viewport size: {width_px}x{height_px}"));
    }
    Ok((width_px, height_px))
}

fn create_back_buffers(
    ctx: &ComPtr<d2d::ID2D1DeviceContext5>,
    width_px: i32,
    height_px: i32,
) -> Result<(ComPtr<d2d::ID2D1Bitmap1>, ComPtr<d2d::ID2D1Bitmap1>, Vec<u8>), String> {
    let width: u32 = width_px
        .try_into()
        .map_err(|_| "Viewport width out of range".to_owned())?;
    let height: u32 = height_px
        .try_into()
        .map_err(|_| "Viewport height out of range".to_owned())?;
    let size = d2d::D2D1_SIZE_U { width, height };

    let target = d2d::ctx_create_bitmap(ctx, size, None, d2d::D2D1_BITMAP_OPTIONS_TARGET).map_err(|err| err.message())?;
    let readback = d2d::ctx_create_bitmap(
        ctx,
        size,
        None,
        d2d::D2D1_BITMAP_OPTIONS_CPU_READ | d2d::D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    )
    .map_err(|err| err.message())?;

    let len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Back buffer size overflow".to_owned())?;
    let bgra = vec![0u8; len];

    Ok((target, readback, bgra))
}

fn to_d2d_color(color: Color) -> d2d::D2D1_COLOR_F {
    d2d::D2D1_COLOR_F {
        r: (color.r as f32) / 255.0,
        g: (color.g as f32) / 255.0,
        b: (color.b as f32) / 255.0,
        a: (color.a as f32) / 255.0,
    }
}
