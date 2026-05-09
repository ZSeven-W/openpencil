#include "bindings.h"
#include "rust_resource_provider.h"

#include "include/core/SkCanvas.h"
#include "include/svg/SkSVGCanvas.h"

#include "modules/svg/include/SkSVGCircle.h"
#include "modules/svg/include/SkSVGClipPath.h"
#include "modules/svg/include/SkSVGContainer.h"
#include "modules/svg/include/SkSVGDefs.h"
#include "modules/svg/include/SkSVGDOM.h"
#include "modules/svg/include/SkSVGEllipse.h"
#include "modules/svg/include/SkSVGFe.h"
#include "modules/svg/include/SkSVGFeBlend.h"
#include "modules/svg/include/SkSVGFeColorMatrix.h"
#include "modules/svg/include/SkSVGFeComponentTransfer.h"
#include "modules/svg/include/SkSVGFeComposite.h"
#include "modules/svg/include/SkSVGFeDisplacementMap.h"
#include "modules/svg/include/SkSVGFeFlood.h"
#include "modules/svg/include/SkSVGFeGaussianBlur.h"
#include "modules/svg/include/SkSVGFeImage.h"
#include "modules/svg/include/SkSVGFeLighting.h"
#include "modules/svg/include/SkSVGFeLightSource.h"
#include "modules/svg/include/SkSVGFeMerge.h"
#include "modules/svg/include/SkSVGFeMorphology.h"
#include "modules/svg/include/SkSVGFeOffset.h"
#include "modules/svg/include/SkSVGFeTurbulence.h"
#include "modules/svg/include/SkSVGFilter.h"
#include "modules/svg/include/SkSVGG.h"
#include "modules/svg/include/SkSVGGradient.h"
#include "modules/svg/include/SkSVGImage.h"
#include "modules/svg/include/SkSVGLine.h"
#include "modules/svg/include/SkSVGLinearGradient.h"
#include "modules/svg/include/SkSVGMask.h"
#include "modules/svg/include/SkSVGNode.h"
#include "modules/svg/include/SkSVGPath.h"
#include "modules/svg/include/SkSVGPattern.h"
#include "modules/svg/include/SkSVGPoly.h"
#include "modules/svg/include/SkSVGRadialGradient.h"
#include "modules/svg/include/SkSVGRect.h"
#include "modules/svg/include/SkSVGRenderContext.h"
#include "modules/svg/include/SkSVGShape.h"
#include "modules/svg/include/SkSVGStop.h"
#include "modules/svg/include/SkSVGSVG.h"
#include "modules/svg/include/SkSVGText.h"
#include "modules/svg/include/SkSVGTypes.h"
#include "modules/svg/include/SkSVGUse.h"
#include "modules/svg/include/SkSVGValue.h"

#include "include/core/SkStream.h"

extern "C" SkSVGDOM* C_SkSVGDOM_MakeFromStream(
    SkStream& stream,
    RustResourceProvider* provider) 
{
    auto builder = SkSVGDOM::Builder();
    builder.setFontManager(provider->fontMgr());
    builder.setResourceProvider(sp(provider));
    return builder.make(stream).release();
}

extern "C" void C_SkSVGDOM_setContainerSize(SkSVGDOM* self, const SkSize& size){
    self->setContainerSize(size);
}

extern "C" SkSVGSVG* C_SkSVGDOM_getRoot(const SkSVGDOM* self){
    return self->getRoot();
}

extern "C" void C_SkSVGTypes(
    SkSVGFeComponentTransfer*,
    SkSVGFeFlood*,
    SkSVGFeLighting*,
    SkSVGFeLightSource*,
    SkSVGFeMerge*,
    SkSVGG*,
    SkSVGHiddenContainer*,
    SkSVGText*,
    SkSVGTSpan*,
    SkSVGValue*,
    SkSVGDefs*
) {};

extern "C" SkSize C_SkSVGSVG_intrinsicSize(const SkSVGSVG* self) {
    return self->intrinsicSize(SkSVGLengthContext(SkSize::Make(0, 0)));
}

extern "C" bool C_SkSVGSVG_parseAndSetAttribute(SkSVGSVG* self, const char* name, const char* value){
    return self->parseAndSetAttribute(name, value);
}

#define SVG_PRES_ATTR(attr_name, attr_type, attr_inheritable)                    \
extern "C" bool C_SkSVGNode_has##attr_name(const SkSVGNode& self) {              \
    return self.get##attr_name().isValue();                                      \
}                                                                                \
extern "C" const attr_type* C_SkSVGNode_get##attr_name(const SkSVGNode& self) {  \
    return &*self.get##attr_name();                                              \
}                                                                                \
extern "C" void C_SkSVGNode_set##attr_name(SkSVGNode* self, const attr_type x) { \
    return self->set##attr_name(SkSVGProperty<attr_type, attr_inheritable>(x));  \
}                                                                                \

#define SVG_PRES_REF_ATTR(attr_name, attr_type, attr_inheritable)                 \
extern "C" bool C_SkSVGNode_has##attr_name(const SkSVGNode& self) {               \
    return self.get##attr_name().isValue();                                       \
}                                                                                 \
extern "C" const attr_type* C_SkSVGNode_get##attr_name(const SkSVGNode& self) {   \
    return &*self.get##attr_name();                                               \
}                                                                                 \
extern "C" void C_SkSVGNode_set##attr_name(SkSVGNode* self, const attr_type& x) { \
    return self->set##attr_name(SkSVGProperty<attr_type, attr_inheritable>(x));   \
}                                                                                 \

#define SVG_ATTRIBUTE_ARRAY(type, attr_name, attr_type)                    \
extern "C" size_t C_##type##_get##attr_name##Count(const type& self) {     \
    return self.get##attr_name().size();                                   \
}                                                                          \
extern "C" attr_type* C_##type##_get##attr_name(const type& self) {        \
    return self.get##attr_name().data();                                   \
}                                                                          \

#define SVG_ATTRIBUTE(type, attr_name, attr_type)                          \
extern "C" const attr_type* C_##type##_get##attr_name(const type& self) {  \
    return &self.get##attr_name();                                         \
}                                                                          \
extern "C" void C_##type##_set##attr_name(type* self, const attr_type x) { \
    return self->set##attr_name(x);                                        \
}                                                                          \

#define SVG_OPTIONAL_ATTRIBUTE(type, attr_name, attr_type)                 \
extern "C" bool C_##type##_has##attr_name(const type& self) {              \
    return self.get##attr_name().has_value();                              \
}                                                                          \
extern "C" const attr_type* C_##type##_get##attr_name(const type& self) {  \
    return &*self.get##attr_name();                                        \
}                                                                          \
extern "C" void C_##type##_set##attr_name(type* self, const attr_type x) { \
    return self->set##attr_name(x);                                        \
}                                                                          \

SVG_ATTRIBUTE(SkSVGCircle, Cx, SkSVGLength);
SVG_ATTRIBUTE(SkSVGCircle, Cy, SkSVGLength);
SVG_ATTRIBUTE(SkSVGCircle, R , SkSVGLength);

SVG_ATTRIBUTE(SkSVGClipPath, ClipPathUnits, SkSVGObjectBoundingBoxUnits);

SVG_ATTRIBUTE(SkSVGEllipse, Cx, SkSVGLength);
SVG_ATTRIBUTE(SkSVGEllipse, Cy, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGEllipse, Rx, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGEllipse, Ry, SkSVGLength);

SVG_ATTRIBUTE(SkSVGFe, In, SkSVGFeInputType);
SVG_ATTRIBUTE(SkSVGFe, Result, SkSVGStringType);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFe, X, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFe, Y, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFe, Width, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFe, Height, SkSVGLength);

SVG_ATTRIBUTE(SkSVGFeBlend, Mode, SkSVGFeBlend::Mode);
SVG_ATTRIBUTE(SkSVGFeBlend, In2, SkSVGFeInputType);

SVG_ATTRIBUTE(SkSVGFeColorMatrix, Type, SkSVGFeColorMatrixType);
SVG_ATTRIBUTE_ARRAY(SkSVGFeColorMatrix, Values, const SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeFunc, Amplitude  , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeFunc, Exponent   , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeFunc, Intercept  , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeFunc, Offset     , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeFunc, Slope      , SkSVGNumberType);
SVG_ATTRIBUTE_ARRAY(SkSVGFeFunc, TableValues, const SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeFunc, Type       , SkSVGFeFuncType);

SVG_ATTRIBUTE(SkSVGFeComposite, In2, SkSVGFeInputType);
SVG_ATTRIBUTE(SkSVGFeComposite, K1, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeComposite, K2, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeComposite, K3, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeComposite, K4, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeComposite, Operator, SkSVGFeCompositeOperator);

SVG_ATTRIBUTE(SkSVGFeDisplacementMap, In2             , SkSVGFeInputType);
SVG_ATTRIBUTE(SkSVGFeDisplacementMap, XChannelSelector, SkSVGFeDisplacementMap::ChannelSelector);
SVG_ATTRIBUTE(SkSVGFeDisplacementMap, YChannelSelector, SkSVGFeDisplacementMap::ChannelSelector);
SVG_ATTRIBUTE(SkSVGFeDisplacementMap, Scale           , SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeGaussianBlur, StdDeviation, SkSVGFeGaussianBlur::StdDeviation);

SVG_ATTRIBUTE(SkSVGFeImage, Href               , SkSVGIRI                );
SVG_ATTRIBUTE(SkSVGFeImage, PreserveAspectRatio, SkSVGPreserveAspectRatio);

SVG_ATTRIBUTE(SkSVGFeLighting, SurfaceScale, SkSVGNumberType);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFeLighting, KernelUnitLength, SkSVGFeLighting::KernelUnitLength);

SVG_ATTRIBUTE(SkSVGFeSpecularLighting, SpecularConstant, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpecularLighting, SpecularExponent, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeDiffuseLighting, DiffuseConstant, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeDistantLight, Azimuth  , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeDistantLight, Elevation, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFePointLight, X, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFePointLight, Y, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFePointLight, Z, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeSpotLight, X               , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, Y               , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, Z               , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, PointsAtX       , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, PointsAtY       , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, PointsAtZ       , SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeSpotLight, SpecularExponent, SkSVGNumberType);
SVG_OPTIONAL_ATTRIBUTE(SkSVGFeSpotLight, LimitingConeAngle, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeMergeNode, In, SkSVGFeInputType);

SVG_ATTRIBUTE(SkSVGFeMorphology, Operator, SkSVGFeMorphology::Operator);
SVG_ATTRIBUTE(SkSVGFeMorphology, Radius  , SkSVGFeMorphology::Radius  );

SVG_ATTRIBUTE(SkSVGFeOffset, Dx, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeOffset, Dy, SkSVGNumberType);

SVG_ATTRIBUTE(SkSVGFeTurbulence, BaseFrequency, SkSVGFeTurbulenceBaseFrequency);
SVG_ATTRIBUTE(SkSVGFeTurbulence, NumOctaves, SkSVGIntegerType);
SVG_ATTRIBUTE(SkSVGFeTurbulence, Seed, SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGFeTurbulence, TurbulenceType, SkSVGFeTurbulenceType);

SVG_ATTRIBUTE(SkSVGFilter, X, SkSVGLength);
SVG_ATTRIBUTE(SkSVGFilter, Y, SkSVGLength);
SVG_ATTRIBUTE(SkSVGFilter, Width, SkSVGLength);
SVG_ATTRIBUTE(SkSVGFilter, Height, SkSVGLength);
SVG_ATTRIBUTE(SkSVGFilter, FilterUnits, SkSVGObjectBoundingBoxUnits);
SVG_ATTRIBUTE(SkSVGFilter, PrimitiveUnits, SkSVGObjectBoundingBoxUnits);

SVG_ATTRIBUTE(SkSVGGradient, Href, SkSVGIRI);
SVG_ATTRIBUTE(SkSVGGradient, GradientTransform, SkSVGTransformType);
SVG_ATTRIBUTE(SkSVGGradient, SpreadMethod, SkSVGSpreadMethod);
SVG_ATTRIBUTE(SkSVGGradient, GradientUnits, SkSVGObjectBoundingBoxUnits);

SVG_ATTRIBUTE(SkSVGImage, X                  , SkSVGLength             );
SVG_ATTRIBUTE(SkSVGImage, Y                  , SkSVGLength             );
SVG_ATTRIBUTE(SkSVGImage, Width              , SkSVGLength             );
SVG_ATTRIBUTE(SkSVGImage, Height             , SkSVGLength             );
SVG_ATTRIBUTE(SkSVGImage, Href               , SkSVGIRI                );
SVG_ATTRIBUTE(SkSVGImage, PreserveAspectRatio, SkSVGPreserveAspectRatio);

SVG_ATTRIBUTE(SkSVGLine, X1, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLine, Y1, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLine, X2, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLine, Y2, SkSVGLength);

SVG_ATTRIBUTE(SkSVGLinearGradient, X1, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLinearGradient, Y1, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLinearGradient, X2, SkSVGLength);
SVG_ATTRIBUTE(SkSVGLinearGradient, Y2, SkSVGLength);

SVG_ATTRIBUTE(SkSVGMask, X     , SkSVGLength);
SVG_ATTRIBUTE(SkSVGMask, Y     , SkSVGLength);
SVG_ATTRIBUTE(SkSVGMask, Width , SkSVGLength);
SVG_ATTRIBUTE(SkSVGMask, Height, SkSVGLength);
SVG_ATTRIBUTE(SkSVGMask, MaskUnits, SkSVGObjectBoundingBoxUnits);
SVG_ATTRIBUTE(SkSVGMask, MaskContentUnits, SkSVGObjectBoundingBoxUnits);

SVG_ATTRIBUTE(SkSVGPath, Path, SkPath);

SVG_ATTRIBUTE(SkSVGPattern, Href, SkSVGIRI);
SVG_OPTIONAL_ATTRIBUTE(SkSVGPattern, X               , SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGPattern, Y               , SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGPattern, Width           , SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGPattern, Height          , SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGPattern, PatternTransform, SkSVGTransformType);

SVG_ATTRIBUTE_ARRAY(SkSVGPoly, Points, const SkPoint);

SVG_ATTRIBUTE(SkSVGRadialGradient, Cx, SkSVGLength);
SVG_ATTRIBUTE(SkSVGRadialGradient, Cy, SkSVGLength);
SVG_ATTRIBUTE(SkSVGRadialGradient, R,  SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGRadialGradient, Fx, SkSVGLength)
SVG_OPTIONAL_ATTRIBUTE(SkSVGRadialGradient, Fy, SkSVGLength)

SVG_ATTRIBUTE(SkSVGRect, X     , SkSVGLength);
SVG_ATTRIBUTE(SkSVGRect, Y     , SkSVGLength);
SVG_ATTRIBUTE(SkSVGRect, Width , SkSVGLength);
SVG_ATTRIBUTE(SkSVGRect, Height, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGRect, Rx, SkSVGLength);
SVG_OPTIONAL_ATTRIBUTE(SkSVGRect, Ry, SkSVGLength);

SVG_ATTRIBUTE(SkSVGStop, Offset, SkSVGLength);

SVG_ATTRIBUTE(SkSVGSVG, X                  , SkSVGLength);
SVG_ATTRIBUTE(SkSVGSVG, Y                  , SkSVGLength);
SVG_ATTRIBUTE(SkSVGSVG, Width              , SkSVGLength);
SVG_ATTRIBUTE(SkSVGSVG, Height             , SkSVGLength);
SVG_ATTRIBUTE(SkSVGSVG, PreserveAspectRatio, SkSVGPreserveAspectRatio);
SVG_OPTIONAL_ATTRIBUTE(SkSVGSVG, ViewBox, SkSVGViewBoxType);

SVG_ATTRIBUTE_ARRAY(SkSVGTextContainer, X, const SkSVGLength);
SVG_ATTRIBUTE_ARRAY(SkSVGTextContainer, Y, const SkSVGLength);
SVG_ATTRIBUTE_ARRAY(SkSVGTextContainer, Dx, const SkSVGLength);
SVG_ATTRIBUTE_ARRAY(SkSVGTextContainer, Dy, const SkSVGLength);
SVG_ATTRIBUTE_ARRAY(SkSVGTextContainer, Rotate, const SkSVGNumberType);
SVG_ATTRIBUTE(SkSVGTextContainer, XmlSpace, SkSVGXmlSpace);

SVG_ATTRIBUTE(SkSVGTextLiteral, Text, SkSVGStringType);

SVG_ATTRIBUTE(SkSVGTextPath, Href       , SkSVGIRI   );
SVG_ATTRIBUTE(SkSVGTextPath, StartOffset, SkSVGLength);

SVG_ATTRIBUTE(SkSVGUse, X   , SkSVGLength);
SVG_ATTRIBUTE(SkSVGUse, Y   , SkSVGLength);
SVG_ATTRIBUTE(SkSVGUse, Href, SkSVGIRI   );

extern "C" void C_SkSVGIRI_Construct(SkSVGIRI* uninitialized) {
    new(uninitialized)SkSVGIRI();
}

extern "C" void C_SkSVGIRI_Construct1(SkSVGIRI* uninitialized, const SkSVGIRI::Type t, const SkSVGStringType& iri) {
    new(uninitialized)SkSVGIRI(t, iri);
}

extern "C" void C_SkSVGFuncIRI_Construct(SkSVGFuncIRI* uninitialized) {
    new(uninitialized)SkSVGFuncIRI();
}

extern "C" void C_SkSVGFuncIRI_Construct1(SkSVGFuncIRI* uninitialized, const SkSVGIRI& iri) {
    new(uninitialized)SkSVGFuncIRI(SkSVGIRI(iri));
}

extern "C" void C_SkSVGPaint_Construct(SkSVGPaint* uninitialized) {
    new(uninitialized)SkSVGPaint();
}

extern "C" void C_SkSVGPaint_Construct1(SkSVGPaint* uninitialized, const SkSVGColor& color) {
    new(uninitialized)SkSVGPaint(color);
}

extern "C" void C_SkSVGColor_Construct(SkSVGColor* uninitialized) {
    new(uninitialized)SkSVGColor(SkSVGColor::Type::kCurrentColor, std::vector<SkString>());
}

extern "C" void C_SkSVGColor_Construct1(SkSVGColor* uninitialized, const SkSVGColorType color) {
    new(uninitialized)SkSVGColor(color);
}

// Hacky way to access the SkSVGContainer::fChildren property (should be safe)
class SkSVGContainerAccessor : public SkSVGContainer {
    public:
        int childrenCount() const {
            return fChildren.size();
        }

        const sk_sp<SkSVGNode>* children() const {
            return fChildren.data();
        }
};

extern "C" void C_SkSVGContainer_appendChild(SkSVGContainer* self, SkSVGNode* node) {
    self->appendChild(sk_sp<SkSVGNode>(node));
}

extern "C" int C_SkSVGContainer_childrenCount(const SkSVGContainer& self) {
    return static_cast<const SkSVGContainerAccessor&>(self).childrenCount();
}

// Getting mutable child references from a non-mutable reference seems unsafe, perhaps we should split this into two methods?
extern "C" const sk_sp<SkSVGNode>* C_SkSVGContainer_children(const SkSVGContainer& self) {
    return static_cast<const SkSVGContainerAccessor&>(self).children();
}

extern "C" void C_SkSVGTransformableNode_setTransform(SkSVGTransformableNode* self, const SkMatrix& value) {
    self->setTransform(value);
}

extern "C" SkSVGTag C_SkSVGNode_tag(const SkSVGNode& self) {
    return self.tag();
}

extern "C" void C_SkSVGIRI_destruct(SkSVGIRI* self) {
    self->~SkSVGIRI();
}

extern "C" void C_SkSVGFuncIRI_destruct(SkSVGFuncIRI* self) {
    self->~SkSVGFuncIRI();
}

extern "C" void C_SkSVGPaint_destruct(SkSVGPaint* self) {
    self->~SkSVGPaint();
}

extern "C" void C_SkSVGColor_destruct(SkSVGColor* self) {
    self->~SkSVGColor();
}

// inherited
SVG_PRES_ATTR(ClipRule                 , SkSVGFillRule,   true)
SVG_PRES_ATTR(Color                    , SkSVGColorType,  true)
SVG_PRES_ATTR(ColorInterpolation       , SkSVGColorspace, true)
SVG_PRES_ATTR(ColorInterpolationFilters, SkSVGColorspace, true)
SVG_PRES_ATTR(FillRule                 , SkSVGFillRule,   true)
SVG_PRES_REF_ATTR(Fill                 , SkSVGPaint,      true)
SVG_PRES_ATTR(FillOpacity              , SkSVGNumberType, true)
SVG_PRES_ATTR(FontFamily               , SkSVGFontFamily, true)
SVG_PRES_ATTR(FontSize                 , SkSVGFontSize,   true)
SVG_PRES_ATTR(FontStyle                , SkSVGFontStyle,  true)
SVG_PRES_ATTR(FontWeight               , SkSVGFontWeight, true)
SVG_PRES_REF_ATTR(Stroke               , SkSVGPaint,      true)
SVG_PRES_ATTR(StrokeLineCap            , SkSVGLineCap,    true)
SVG_PRES_ATTR(StrokeLineJoin           , SkSVGLineJoin,   true)
SVG_PRES_ATTR(StrokeMiterLimit         , SkSVGNumberType, true)
SVG_PRES_ATTR(StrokeOpacity            , SkSVGNumberType, true)
SVG_PRES_ATTR(StrokeWidth              , SkSVGLength,     true)
SVG_PRES_ATTR(TextAnchor               , SkSVGTextAnchor, true)
SVG_PRES_ATTR(Visibility               , SkSVGVisibility, true)

// not inherited
SVG_PRES_REF_ATTR(ClipPath             , SkSVGFuncIRI   , false)
SVG_PRES_ATTR(Display                  , SkSVGDisplay   , false)
SVG_PRES_REF_ATTR(Mask                 , SkSVGFuncIRI   , false)
SVG_PRES_REF_ATTR(Filter               , SkSVGFuncIRI   , false)
SVG_PRES_ATTR(Opacity                  , SkSVGNumberType, false)
SVG_PRES_REF_ATTR(StopColor            , SkSVGColor     , false)
SVG_PRES_ATTR(StopOpacity              , SkSVGNumberType, false)
SVG_PRES_REF_ATTR(FloodColor           , SkSVGColor     , false)
SVG_PRES_ATTR(FloodOpacity             , SkSVGNumberType, false)
SVG_PRES_REF_ATTR(LightingColor        , SkSVGColor     , false)

#define SVG_MAKE(type)               \
extern "C" type* C_##type##_Make() { \
    return type::Make().release();   \
}

SVG_MAKE(SkSVGFeBlend)
SVG_MAKE(SkSVGFeColorMatrix)
SVG_MAKE(SkSVGFeComposite)
SVG_MAKE(SkSVGFeDisplacementMap)
SVG_MAKE(SkSVGFeFlood)

extern "C" SkSVGFeFunc* C_SkSVGFeFunc_MakeFuncA() {
    return SkSVGFeFunc::MakeFuncA().release();
}
extern "C" SkSVGFeFunc* C_SkSVGFeFunc_MakeFuncR() {
    return SkSVGFeFunc::MakeFuncR().release();
}
extern "C" SkSVGFeFunc* C_SkSVGFeFunc_MakeFuncG() {
    return SkSVGFeFunc::MakeFuncG().release();
}
extern "C" SkSVGFeFunc* C_SkSVGFeFunc_MakeFuncB() {
    return SkSVGFeFunc::MakeFuncB().release();
}

SVG_MAKE(SkSVGFeComponentTransfer)
SVG_MAKE(SkSVGFeGaussianBlur)
SVG_MAKE(SkSVGFeImage)

SVG_MAKE(SkSVGFeDistantLight)
SVG_MAKE(SkSVGFePointLight)
SVG_MAKE(SkSVGFeSpotLight)

SVG_MAKE(SkSVGFeSpecularLighting)
SVG_MAKE(SkSVGFeDiffuseLighting)

SVG_MAKE(SkSVGFeMergeNode)
SVG_MAKE(SkSVGFeMerge)

SVG_MAKE(SkSVGFeMorphology)
SVG_MAKE(SkSVGFeOffset)
SVG_MAKE(SkSVGFeTurbulence)

SVG_MAKE(SkSVGLinearGradient)
SVG_MAKE(SkSVGRadialGradient)

SVG_MAKE(SkSVGCircle)
SVG_MAKE(SkSVGEllipse)
SVG_MAKE(SkSVGLine)
SVG_MAKE(SkSVGPath)

extern "C" SkSVGPoly* C_SkSVGPoly_MakePolygon() {
    return SkSVGPoly::MakePolygon().release();
}

extern "C" SkSVGPoly* C_SkSVGPoly_MakePolyline() {
    return SkSVGPoly::MakePolyline().release();
}

SVG_MAKE(SkSVGRect)

SVG_MAKE(SkSVGClipPath)
SVG_MAKE(SkSVGDefs)
SVG_MAKE(SkSVGFilter)
SVG_MAKE(SkSVGG)
SVG_MAKE(SkSVGImage)
SVG_MAKE(SkSVGMask)
SVG_MAKE(SkSVGPattern)
SVG_MAKE(SkSVGStop)

extern "C" SkSVGSVG* C_SkSVGSVG_Make(SkSVGSVG::Type t) {
    return SkSVGSVG::Make(t).release();
}

SVG_MAKE(SkSVGText)
SVG_MAKE(SkSVGTSpan)
SVG_MAKE(SkSVGTextLiteral)
SVG_MAKE(SkSVGTextPath)

SVG_MAKE(SkSVGUse)
