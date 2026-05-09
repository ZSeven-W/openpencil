use super::{
    fe, pattern::Pattern, svg_::Svg, Circle, ClipPath, ColorSpace, DebugAttributes, Defs, Display,
    Ellipse, Fill, FillRule, Filter, FontFamily, FontSize, FontStyle, FontWeight, Image, IriFunc,
    Length, Line, LineCap, LineJoin, LinearGradient, Mask, Paint, Path, Poly, RadialGradient, Rect,
    Stop, TSpan, Text, TextAnchor, TextLiteral, TextPath, Use, Visibility, G,
};
use crate::{prelude::*, scalar, Color};
use skia_bindings as sb;

pub type NodeTag = sb::SkSVGTag;

pub type Node = RCHandle<sb::SkSVGNode>;

impl NativeRefCountedBase for sb::SkSVGNode {
    type Base = sb::SkRefCntBase;
}

impl DebugAttributes for Node {
    const NAME: &'static str = "Node";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        builder
            .field("clip_rule", &self.clip_rule())
            .field("color_interpolation", &self.color_interpolation())
            .field(
                "color_interpolation_filters",
                &self.color_interpolation_filters(),
            )
            .field("color", &self.color())
            .field("fill_rule", &self.fill_rule())
            .field("fill", &self.fill())
            .field("fill_opacity", &self.fill_opacity())
            .field("font_family", &self.font_family())
            .field("font_size", &self.font_size())
            .field("font_style", &self.font_style())
            .field("font_weight", &self.font_weight())
            .field("stroke", &self.stroke())
            .field("stroke_line_cap", &self.stroke_line_cap())
            .field("stroke_line_join", &self.stroke_line_join())
            .field("stroke_miter_limit", &self.stroke_miter_limit())
            .field("stroke_opacity", &self.stroke_opacity())
            .field("stroke_width", &self.stroke_width())
            .field("text_anchor", &self.text_anchor())
            .field("visibility", &self.visibility())
            .field("clip_path", &self.clip_path())
            .field("display", &self.display())
            .field("mask", &self.mask())
            .field("filter", &self.filter())
            .field("opacity", &self.opacity())
            .field("stop_color", &self.stop_color())
            .field("stop_opacity", &self.stop_opacity())
            .field("flood_color", &self.flood_color())
            .field("flood_opacity", &self.flood_opacity())
            .field("lighting_color", &self.lighting_color());
    }
}

impl From<TypedNode> for Node {
    fn from(value: TypedNode) -> Self {
        value.into_node()
    }
}

impl Node {
    pub fn tag(&self) -> NodeTag {
        unsafe { sb::C_SkSVGNode_tag(self.native()) }
    }

    // TODO: wrap appendChild()
    // TODO: wrap render(), asPaint(), asPath(), objectBoundingBox()
    // TODO: wrap setAttribute().
    // TODO: wrap parseAndSetAttribute()

    pub fn typed(self) -> TypedNode {
        TypedNode::from_ptr(self.into_ptr())
    }

    skia_svg_macros::attrs! {
        SkSVGNode => {
            // inherited
            clip_rule?: FillRule [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGFillRule { fType: value }],
            color_interpolation?: ColorSpace [get(value) => value, set(value) => value],
            color_interpolation_filters?: ColorSpace [get(value) => value, set(value) => value],
            color?: Color [get(value) => value.map(Color::from_native_ref), set(value) => value.into_native()],
            fill_rule?: FillRule [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGFillRule { fType: value }],
            fill?: Paint [get(value) => value.map(Paint::from_native_ref), set(value) => value.native()],
            *fill_opacity?: scalar [get(value) => value, set(value) => value],
            font_family?: FontFamily [get(value) => value.map(FontFamily::from_native_ref), set(value) => value.into_native()],
            font_size?: FontSize [get(value) => value.map(FontSize::from_native_ref), set(value) => value.into_native()],
            font_style?: FontStyle [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGFontStyle { fType: value }],
            font_weight?: FontWeight [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGFontWeight { fType: value }],
            stroke?: Paint [get(value) => value.map(Paint::from_native_ref), set(value) => value.native()],
            stroke_line_cap?: LineCap [get(value) => value, set(value) => value],
            stroke_line_join?: LineJoin [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGLineJoin { fType: value }],
            *stroke_miter_limit?: scalar [get(value) => value, set(value) => value],
            *stroke_opacity?: scalar [get(value) => value, set(value) => value],
            stroke_width?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            text_anchor?: TextAnchor [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGTextAnchor { fType: value }],
            visibility?: Visibility [get(value) => value.map(|value| &value.fType), set(value) => sb::SkSVGVisibility { fType: value }],

            // not inherited
            clip_path?: IriFunc [get(value) => value.map(IriFunc::from_native_ref), set(value) => value.native()],
            display?: Display [get(value) => value, set(value) => value],
            mask?: IriFunc [get(value) => value.map(IriFunc::from_native_ref), set(value) => value.native()],
            filter?: IriFunc [get(value) => value.map(IriFunc::from_native_ref), set(value) => value.native()],
            *opacity?: scalar [get(value) => value, set(value) => value],
            stop_color?: Fill [get(value) => value.map(Fill::from_native_ref), set(value) => value.native()],
            *stop_opacity?: scalar [get(value) => value, set(value) => value],
            flood_color?: Fill [get(value) => value.map(Fill::from_native_ref), set(value) => value.native()],
            *flood_opacity?: scalar [get(value) => value, set(value) => value],
            lighting_color?: Fill [get(value) => value.map(Fill::from_native_ref), set(value) => value.native()]
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypedNode {
    Circle(Circle),
    ClipPath(ClipPath),
    Defs(Defs),
    Ellipse(Ellipse),
    FeBlend(fe::Blend),
    FeColorMatrix(fe::ColorMatrix),
    FeComponentTransfer(fe::ComponentTransfer),
    FeComposite(fe::Composite),
    FeDiffuseLighting(fe::DiffuseLighting),
    FeDisplacementMap(fe::DisplacementMap),
    FeDistantLight(fe::DistantLight),
    FeFlood(fe::Flood),
    FeFuncA(fe::Func),
    FeFuncR(fe::Func),
    FeFuncG(fe::Func),
    FeFuncB(fe::Func),
    FeGaussianBlur(fe::GaussianBlur),
    FeImage(fe::Image),
    FeMerge(fe::Merge),
    FeMergeNode(fe::MergeNode),
    FeMorphology(fe::Morphology),
    FeOffset(fe::Offset),
    FePointLight(fe::PointLight),
    FeSpecularLighting(fe::SpecularLighting),
    FeSpotLight(fe::SpotLight),
    FeTurbulence(fe::Turbulence),
    Filter(Filter),
    G(G),
    Image(Image),
    Line(Line),
    LinearGradient(LinearGradient),
    Mask(Mask),
    Path(Path),
    Pattern(Pattern),
    Polygon(Poly),
    Polyline(Poly),
    RadialGradient(RadialGradient),
    Rect(Rect),
    Stop(Stop),
    Svg(Svg),
    Text(Text),
    TextLiteral(TextLiteral),
    TextPath(TextPath),
    TSpan(TSpan),
    Use(Use),
}

impl From<Node> for TypedNode {
    fn from(node: Node) -> Self {
        node.typed()
    }
}

impl TypedNode {
    pub(crate) fn from_ptr(ptr: *mut sb::SkSVGNode) -> Self {
        Self::from_ptr_opt(ptr).unwrap()
    }

    pub fn into_node(self) -> Node {
        let node_ptr = match self {
            TypedNode::Circle(node) => node.into_ptr() as *mut _,
            TypedNode::ClipPath(node) => node.into_ptr() as *mut _,
            TypedNode::Defs(node) => node.into_ptr() as *mut _,
            TypedNode::Ellipse(node) => node.into_ptr() as *mut _,
            TypedNode::FeBlend(node) => node.into_ptr() as *mut _,
            TypedNode::FeColorMatrix(node) => node.into_ptr() as *mut _,
            TypedNode::FeComponentTransfer(node) => node.into_ptr() as *mut _,
            TypedNode::FeComposite(node) => node.into_ptr() as *mut _,
            TypedNode::FeDiffuseLighting(node) => node.into_ptr() as *mut _,
            TypedNode::FeDisplacementMap(node) => node.into_ptr() as *mut _,
            TypedNode::FeDistantLight(node) => node.into_ptr() as *mut _,
            TypedNode::FeFlood(node) => node.into_ptr() as *mut _,
            TypedNode::FeFuncA(node) => node.into_ptr() as *mut _,
            TypedNode::FeFuncR(node) => node.into_ptr() as *mut _,
            TypedNode::FeFuncG(node) => node.into_ptr() as *mut _,
            TypedNode::FeFuncB(node) => node.into_ptr() as *mut _,
            TypedNode::FeGaussianBlur(node) => node.into_ptr() as *mut _,
            TypedNode::FeImage(node) => node.into_ptr() as *mut _,
            TypedNode::FeMerge(node) => node.into_ptr() as *mut _,
            TypedNode::FeMergeNode(node) => node.into_ptr() as *mut _,
            TypedNode::FeMorphology(node) => node.into_ptr() as *mut _,
            TypedNode::FeOffset(node) => node.into_ptr() as *mut _,
            TypedNode::FePointLight(node) => node.into_ptr() as *mut _,
            TypedNode::FeSpecularLighting(node) => node.into_ptr() as *mut _,
            TypedNode::FeSpotLight(node) => node.into_ptr() as *mut _,
            TypedNode::FeTurbulence(node) => node.into_ptr() as *mut _,
            TypedNode::Filter(node) => node.into_ptr() as *mut _,
            TypedNode::G(node) => node.into_ptr() as *mut _,
            TypedNode::Image(node) => node.into_ptr() as *mut _,
            TypedNode::Line(node) => node.into_ptr() as *mut _,
            TypedNode::LinearGradient(node) => node.into_ptr() as *mut _,
            TypedNode::Mask(node) => node.into_ptr() as *mut _,
            TypedNode::Path(node) => node.into_ptr() as *mut _,
            TypedNode::Pattern(node) => node.into_ptr() as *mut _,
            TypedNode::Polygon(node) => node.into_ptr() as *mut _,
            TypedNode::Polyline(node) => node.into_ptr() as *mut _,
            TypedNode::RadialGradient(node) => node.into_ptr() as *mut _,
            TypedNode::Rect(node) => node.into_ptr() as *mut _,
            TypedNode::Stop(node) => node.into_ptr() as *mut _,
            TypedNode::Svg(node) => node.into_ptr() as *mut _,
            TypedNode::Text(node) => node.into_ptr() as *mut _,
            TypedNode::TextLiteral(node) => node.into_ptr() as *mut _,
            TypedNode::TextPath(node) => node.into_ptr() as *mut _,
            TypedNode::TSpan(node) => node.into_ptr() as *mut _,
            TypedNode::Use(node) => node.into_ptr() as *mut _,
        };

        Node::from_ptr(node_ptr).unwrap()
    }

    pub(crate) fn from_ptr_opt(ptr: *mut sb::SkSVGNode) -> Option<Self> {
        let tag = unsafe { sb::C_SkSVGNode_tag(ptr as *const _) };

        Some(match tag {
            NodeTag::Circle => Self::Circle(Circle::from_ptr(ptr as *mut _)?),
            NodeTag::ClipPath => Self::ClipPath(ClipPath::from_ptr(ptr as *mut _)?),
            NodeTag::Defs => Self::Defs(Defs::from_ptr(ptr as *mut _)?),
            NodeTag::Ellipse => Self::Ellipse(Ellipse::from_ptr(ptr as *mut _)?),
            NodeTag::FeBlend => Self::FeBlend(fe::Blend::from_ptr(ptr as *mut _)?),
            NodeTag::FeColorMatrix => {
                Self::FeColorMatrix(fe::ColorMatrix::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeComponentTransfer => {
                Self::FeComponentTransfer(fe::ComponentTransfer::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeComposite => Self::FeComposite(fe::Composite::from_ptr(ptr as *mut _)?),
            NodeTag::FeDiffuseLighting => {
                Self::FeDiffuseLighting(fe::DiffuseLighting::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeDisplacementMap => {
                Self::FeDisplacementMap(fe::DisplacementMap::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeDistantLight => {
                Self::FeDistantLight(fe::DistantLight::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeFlood => Self::FeFlood(fe::Flood::from_ptr(ptr as *mut _)?),
            NodeTag::FeFuncA => Self::FeFuncA(fe::Func::from_ptr(ptr as *mut _)?),
            NodeTag::FeFuncR => Self::FeFuncR(fe::Func::from_ptr(ptr as *mut _)?),
            NodeTag::FeFuncG => Self::FeFuncG(fe::Func::from_ptr(ptr as *mut _)?),
            NodeTag::FeFuncB => Self::FeFuncB(fe::Func::from_ptr(ptr as *mut _)?),
            NodeTag::FeGaussianBlur => {
                Self::FeGaussianBlur(fe::GaussianBlur::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeImage => Self::FeImage(fe::Image::from_ptr(ptr as *mut _)?),
            NodeTag::FeMerge => Self::FeMerge(fe::Merge::from_ptr(ptr as *mut _)?),
            NodeTag::FeMergeNode => Self::FeMergeNode(fe::MergeNode::from_ptr(ptr as *mut _)?),
            NodeTag::FeMorphology => Self::FeMorphology(fe::Morphology::from_ptr(ptr as *mut _)?),
            NodeTag::FeOffset => Self::FeOffset(fe::Offset::from_ptr(ptr as *mut _)?),
            NodeTag::FePointLight => Self::FePointLight(fe::PointLight::from_ptr(ptr as *mut _)?),
            NodeTag::FeSpecularLighting => {
                Self::FeSpecularLighting(fe::SpecularLighting::from_ptr(ptr as *mut _)?)
            }
            NodeTag::FeSpotLight => Self::FeSpotLight(fe::SpotLight::from_ptr(ptr as *mut _)?),
            NodeTag::FeTurbulence => Self::FeTurbulence(fe::Turbulence::from_ptr(ptr as *mut _)?),
            NodeTag::Filter => Self::Filter(Filter::from_ptr(ptr as *mut _)?),
            NodeTag::G => Self::G(G::from_ptr(ptr as *mut _)?),
            NodeTag::Image => Self::Image(Image::from_ptr(ptr as *mut _)?),
            NodeTag::Line => Self::Line(Line::from_ptr(ptr as *mut _)?),
            NodeTag::LinearGradient => {
                Self::LinearGradient(LinearGradient::from_ptr(ptr as *mut _)?)
            }
            NodeTag::Mask => Self::Mask(Mask::from_ptr(ptr as *mut _)?),
            NodeTag::Path => Self::Path(Path::from_ptr(ptr as *mut _)?),
            NodeTag::Pattern => Self::Pattern(Pattern::from_ptr(ptr as *mut _)?),
            NodeTag::Polygon => Self::Polygon(Poly::from_ptr(ptr as *mut _)?),
            NodeTag::Polyline => Self::Polyline(Poly::from_ptr(ptr as *mut _)?),
            NodeTag::RadialGradient => {
                Self::RadialGradient(RadialGradient::from_ptr(ptr as *mut _)?)
            }
            NodeTag::Rect => Self::Rect(Rect::from_ptr(ptr as *mut _)?),
            NodeTag::Stop => Self::Stop(Stop::from_ptr(ptr as *mut _)?),
            NodeTag::Svg => Self::Svg(Svg::from_ptr(ptr as *mut _)?),
            NodeTag::Text => Self::Text(Text::from_ptr(ptr as *mut _)?),
            NodeTag::TextLiteral => Self::TextLiteral(TextLiteral::from_ptr(ptr as *mut _)?),
            NodeTag::TextPath => Self::TextPath(TextPath::from_ptr(ptr as *mut _)?),
            NodeTag::TSpan => Self::TSpan(TSpan::from_ptr(ptr as *mut _)?),
            NodeTag::Use => Self::Use(Use::from_ptr(ptr as *mut _)?),
        })
    }
}
