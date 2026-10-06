//! Tool definitions.

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SelectMode {
    Rectangle,
    FreeForm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BrushKind {
    Brush,
    CalligraphyLeft,
    CalligraphyRight,
    Airbrush,
    Oil,
    Crayon,
    Marker,
    NaturalPencil,
    Watercolor,
}

impl BrushKind {
    pub const ALL: [BrushKind; 9] = [
        BrushKind::Brush,
        BrushKind::CalligraphyLeft,
        BrushKind::CalligraphyRight,
        BrushKind::Airbrush,
        BrushKind::Oil,
        BrushKind::Crayon,
        BrushKind::Marker,
        BrushKind::NaturalPencil,
        BrushKind::Watercolor,
    ];
    pub fn name(self) -> &'static str {
        match self {
            BrushKind::Brush => "Brush",
            BrushKind::CalligraphyLeft => "Calligraphy brush 1",
            BrushKind::CalligraphyRight => "Calligraphy brush 2",
            BrushKind::Airbrush => "Airbrush",
            BrushKind::Oil => "Oil brush",
            BrushKind::Crayon => "Crayon",
            BrushKind::Marker => "Marker",
            BrushKind::NaturalPencil => "Natural pencil",
            BrushKind::Watercolor => "Watercolour brush",
        }
    }
    /// Opacity the stroke layer is composited with.
    pub fn layer_opacity(self) -> f32 {
        match self {
            BrushKind::Marker => 0.55,
            BrushKind::Watercolor => 0.45,
            BrushKind::NaturalPencil => 0.85,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    Line,
    Curve,
    Oval,
    Rectangle,
    RoundedRectangle,
    Polygon,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    RightArrow,
    LeftArrow,
    UpArrow,
    DownArrow,
    Star4,
    Star5,
    Star6,
    RoundedCallout,
    OvalCallout,
    CloudCallout,
    Heart,
    Lightning,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 23] = [
        ShapeKind::Line,
        ShapeKind::Curve,
        ShapeKind::Oval,
        ShapeKind::Rectangle,
        ShapeKind::RoundedRectangle,
        ShapeKind::Polygon,
        ShapeKind::Triangle,
        ShapeKind::RightTriangle,
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::RightArrow,
        ShapeKind::LeftArrow,
        ShapeKind::UpArrow,
        ShapeKind::DownArrow,
        ShapeKind::Star4,
        ShapeKind::Star5,
        ShapeKind::Star6,
        ShapeKind::RoundedCallout,
        ShapeKind::OvalCallout,
        ShapeKind::CloudCallout,
        ShapeKind::Heart,
        ShapeKind::Lightning,
    ];
    pub fn name(self) -> &'static str {
        match self {
            ShapeKind::Line => "Line",
            ShapeKind::Curve => "Curve",
            ShapeKind::Oval => "Oval",
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::RoundedRectangle => "Rounded rectangle",
            ShapeKind::Polygon => "Polygon",
            ShapeKind::Triangle => "Triangle",
            ShapeKind::RightTriangle => "Right triangle",
            ShapeKind::Diamond => "Diamond",
            ShapeKind::Pentagon => "Pentagon",
            ShapeKind::Hexagon => "Hexagon",
            ShapeKind::RightArrow => "Right arrow",
            ShapeKind::LeftArrow => "Left arrow",
            ShapeKind::UpArrow => "Up arrow",
            ShapeKind::DownArrow => "Down arrow",
            ShapeKind::Star4 => "Four-point star",
            ShapeKind::Star5 => "Five-point star",
            ShapeKind::Star6 => "Six-point star",
            ShapeKind::RoundedCallout => "Rounded rectangular callout",
            ShapeKind::OvalCallout => "Oval callout",
            ShapeKind::CloudCallout => "Cloud callout",
            ShapeKind::Heart => "Heart",
            ShapeKind::Lightning => "Lightning",
        }
    }
    /// Shapes defined by a bounding box (everything except line/curve/polygon).
    pub fn is_boxed(self) -> bool {
        !matches!(self, ShapeKind::Line | ShapeKind::Curve | ShapeKind::Polygon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeStyle {
    None,
    Solid,
    Crayon,
    Marker,
    Oil,
    NaturalPencil,
    Watercolor,
}

impl ShapeStyle {
    pub const ALL: [ShapeStyle; 7] = [
        ShapeStyle::None,
        ShapeStyle::Solid,
        ShapeStyle::Crayon,
        ShapeStyle::Marker,
        ShapeStyle::Oil,
        ShapeStyle::NaturalPencil,
        ShapeStyle::Watercolor,
    ];
    pub fn outline_name(self) -> &'static str {
        match self {
            ShapeStyle::None => "No outline",
            ShapeStyle::Solid => "Solid colour",
            ShapeStyle::Crayon => "Crayon",
            ShapeStyle::Marker => "Marker",
            ShapeStyle::Oil => "Oil",
            ShapeStyle::NaturalPencil => "Natural pencil",
            ShapeStyle::Watercolor => "Watercolour",
        }
    }
    pub fn fill_name(self) -> &'static str {
        match self {
            ShapeStyle::None => "No fill",
            other => other.outline_name(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tool {
    Select,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush,
    Shape,
}

/// Size presets offered in the "Size" drop-down, per tool family.
pub fn size_presets(tool: Tool) -> &'static [f32] {
    match tool {
        Tool::Pencil => &[1.0, 2.0, 3.0, 4.0],
        Tool::Eraser => &[4.0, 6.0, 8.0, 10.0, 16.0, 24.0],
        Tool::Shape => &[1.0, 3.0, 5.0, 8.0],
        _ => &[1.0, 3.0, 5.0, 8.0, 12.0, 20.0],
    }
}
