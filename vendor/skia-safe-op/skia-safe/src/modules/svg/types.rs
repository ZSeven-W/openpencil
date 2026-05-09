mod color;
mod font;
mod iri;
mod length;
mod paint;
pub mod preserve_aspect_ratio;

pub use self::{
    color::*, font::*, iri::*, length::*, paint::*, preserve_aspect_ratio::PreserveAspectRatio,
};

use skia_bindings as sb;

pub type FillRule = sb::SkSVGFillRule_Type;
variant_name!(FillRule::NonZero);
pub type ColorSpace = sb::SkSVGColorspace;
variant_name!(ColorSpace::LinearRGB);
pub type Display = sb::SkSVGDisplay;
variant_name!(Display::Inline);
pub type LineCap = sb::SkSVGLineCap;
variant_name!(LineCap::Round);
pub type Visibility = sb::SkSVGVisibility_Type;
variant_name!(Visibility::Hidden);
pub type LineJoin = sb::SkSVGLineJoin_Type;
variant_name!(LineJoin::Miter);
pub type TextAnchor = sb::SkSVGTextAnchor_Type;
variant_name!(TextAnchor::Middle);
pub type BoundingBoxUnits = sb::SkSVGObjectBoundingBoxUnits_Type;
variant_name!(BoundingBoxUnits::UserSpaceOnUse);
pub type SpreadMethod = sb::SkSVGSpreadMethod_Type;
variant_name!(SpreadMethod::Reflect);
pub type XmlSpace = sb::SkSVGXmlSpace;
variant_name!(XmlSpace::Preserve);
