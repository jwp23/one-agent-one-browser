mod background;
mod builder;
mod computer;
mod custom_properties;
mod declarations;
mod length;
mod parse;
mod selectors;

use crate::geom::{Color, Edges};
use std::borrow::Cow;

pub use background::{GradientDirection, LinearGradient};
pub use computer::StyleComputer;
pub use custom_properties::CustomProperties;
pub use length::CssLength;

use builder::{CascadePriority, Cascaded, LetterSpacing, StyleBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    Flex,
    Table,
    TableRow,
    TableCell,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Float {
    None,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
}

impl TextTransform {
    pub fn apply<'a>(self, input: &'a str) -> Cow<'a, str> {
        match self {
            TextTransform::None => Cow::Borrowed(input),
            TextTransform::Uppercase => {
                Cow::Owned(input.chars().flat_map(|ch| ch.to_uppercase()).collect())
            }
            TextTransform::Lowercase => {
                Cow::Owned(input.chars().flat_map(|ch| ch.to_lowercase()).collect())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    Px(i32),
    Number(f32),
}

impl LineHeight {
    pub fn resolve_px(self, font_size_px: i32) -> Option<i32> {
        let font_size_px = font_size_px.max(0);
        match self {
            LineHeight::Normal => None,
            LineHeight::Px(px) => Some(px.max(0)),
            LineHeight::Number(multiplier) => {
                let px = (multiplier * (font_size_px as f32)).round() as i32;
                Some(px.max(0))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexJustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexAlignItems {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub position: Position,
    pub float: Float,
    pub custom_properties: CustomProperties,
    pub top_px: Option<CssLength>,
    pub right_px: Option<CssLength>,
    pub bottom_px: Option<CssLength>,
    pub left_px: Option<CssLength>,
    pub opacity: u8,
    pub color: Color,
    pub background_color: Option<Color>,
    pub background_gradient: Option<LinearGradient>,
    pub font_family: FontFamily,
    pub font_size_px: i32,
    pub letter_spacing_px: i32,
    pub bold: bool,
    pub underline: bool,
    pub text_align: TextAlign,
    pub text_transform: TextTransform,
    pub line_height: LineHeight,
    pub margin: Edges,
    pub margin_auto: AutoEdges,
    pub border_width: Edges,
    pub border_style: BorderStyle,
    pub border_color: Color,
    pub border_radius_px: i32,
    pub padding: CssEdges,
    pub width_px: Option<CssLength>,
    pub min_width_px: Option<CssLength>,
    pub max_width_px: Option<CssLength>,
    pub height_px: Option<i32>,
    pub min_height_px: Option<i32>,
    pub flex_justify_content: FlexJustifyContent,
    pub flex_align_items: FlexAlignItems,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: i32,
    pub flex_shrink: i32,
    pub flex_basis_px: Option<i32>,
    pub flex_gap_px: i32,
}

impl ComputedStyle {
    pub fn root_defaults() -> ComputedStyle {
        ComputedStyle {
            display: Display::Block,
            visibility: Visibility::Visible,
            position: Position::Static,
            float: Float::None,
            custom_properties: CustomProperties::default(),
            top_px: None,
            right_px: None,
            bottom_px: None,
            left_px: None,
            opacity: 255,
            color: Color::BLACK,
            background_color: None,
            background_gradient: None,
            font_family: FontFamily::SansSerif,
            font_size_px: 16,
            letter_spacing_px: 0,
            bold: false,
            underline: false,
            text_align: TextAlign::Left,
            text_transform: TextTransform::None,
            line_height: LineHeight::Normal,
            margin: Edges::ZERO,
            margin_auto: AutoEdges::NONE,
            border_width: Edges::ZERO,
            border_style: BorderStyle::None,
            border_color: Color::BLACK,
            border_radius_px: 0,
            padding: CssEdges::ZERO,
            width_px: None,
            min_width_px: None,
            max_width_px: None,
            height_px: None,
            min_height_px: None,
            flex_justify_content: FlexJustifyContent::Start,
            flex_align_items: FlexAlignItems::Start,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0,
            flex_shrink: 1,
            flex_basis_px: None,
            flex_gap_px: 0,
        }
    }

    pub fn inherit_from(parent: &ComputedStyle, display: Display) -> ComputedStyle {
        ComputedStyle {
            display,
            visibility: Visibility::Visible,
            position: Position::Static,
            float: Float::None,
            custom_properties: parent.custom_properties.clone(),
            top_px: None,
            right_px: None,
            bottom_px: None,
            left_px: None,
            opacity: 255,
            color: parent.color,
            background_color: None,
            background_gradient: None,
            font_family: parent.font_family,
            font_size_px: parent.font_size_px,
            letter_spacing_px: parent.letter_spacing_px,
            bold: parent.bold,
            underline: parent.underline,
            text_align: parent.text_align,
            text_transform: parent.text_transform,
            line_height: parent.line_height,
            margin: Edges::ZERO,
            margin_auto: AutoEdges::NONE,
            border_width: Edges::ZERO,
            border_style: BorderStyle::None,
            border_color: parent.color,
            border_radius_px: 0,
            padding: CssEdges::ZERO,
            width_px: None,
            min_width_px: None,
            max_width_px: None,
            height_px: None,
            min_height_px: None,
            flex_justify_content: FlexJustifyContent::Start,
            flex_align_items: FlexAlignItems::Start,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0,
            flex_shrink: 1,
            flex_basis_px: None,
            flex_gap_px: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoEdges {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl AutoEdges {
    pub const NONE: AutoEdges = AutoEdges {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct CssEdges {
    pub top: CssLength,
    pub right: CssLength,
    pub bottom: CssLength,
    pub left: CssLength,
}

impl CssEdges {
    pub const ZERO: CssEdges = CssEdges {
        top: CssLength::Px(0),
        right: CssLength::Px(0),
        bottom: CssLength::Px(0),
        left: CssLength::Px(0),
    };

    pub fn resolve_px(self, reference_width_px: i32) -> Edges {
        Edges {
            top: self.top.resolve_px(reference_width_px),
            right: self.right.resolve_px(reference_width_px),
            bottom: self.bottom.resolve_px(reference_width_px),
            left: self.left.resolve_px(reference_width_px),
        }
    }
}
