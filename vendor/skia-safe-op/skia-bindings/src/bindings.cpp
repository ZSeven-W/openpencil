#include <cassert>
#include <tuple>
#include <vector>
#include <memory>

#include "bindings.h"
// codec/
#include "include/codec/SkEncodedOrigin.h"
#include "include/codec/SkCodec.h"
#include "include/codec/SkCodecAnimation.h"
#include "include/codec/SkEncodedImageFormat.h"
#include "include/codec/SkPixmapUtils.h"

#include "include/codec/SkBmpDecoder.h"
#include "include/codec/SkGifDecoder.h"
#include "include/codec/SkIcoDecoder.h"
#include "include/codec/SkJpegDecoder.h"
#include "include/codec/SkPngDecoder.h"
// #include "include/codec/SkPngRustDecoder.h"
#include "include/codec/SkWbmpDecoder.h"

#if defined(SK_CODEC_DECODES_WEBP)
#include "include/codec/SkWebpDecoder.h"
#endif

// core/
#include "include/core/SkAnnotation.h"
#include "include/core/SkArc.h"
#include "include/core/SkBBHFactory.h"
#include "include/core/SkBlendMode.h"
#include "include/core/SkBitmap.h"
#include "include/core/SkBlurTypes.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkColor.h"
#include "include/core/SkColorFilter.h"
#include "include/core/SkColorTable.h"
#include "include/core/SkCPURecorder.h"
#include "include/core/SkContourMeasure.h"
#include "include/core/SkCoverageMode.h"
#include "include/core/SkCubicMap.h"
#include "include/core/SkDataTable.h"
#include "include/core/SkDrawable.h"
#include "include/core/SkDocument.h"
#include "include/core/SkFlattenable.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontArguments.h"
#include "include/core/SkFontMetrics.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkGraphics.h"
#include "include/core/SkImage.h"
#include "include/core/SkImageFilter.h"
#include "include/core/SkImageGenerator.h"
#include "include/core/SkImageInfo.h"
#include "include/core/SkM44.h"
#include "include/core/SkMaskFilter.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkPathMeasure.h"
#include "include/core/SkPathTypes.h"
#include "include/core/SkPathUtils.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "include/core/SkPixelRef.h"
#include "include/core/SkPoint.h"
#include "include/core/SkPoint3.h"
#include "include/core/SkRecorder.h"
#include "include/core/SkRect.h"
#include "include/core/SkRefCnt.h"
#include "include/core/SkRegion.h"
#include "include/core/SkRRect.h"
#include "include/core/SkRSXform.h"
#include "include/core/SkStream.h"
#include "include/core/SkStrokeRec.h"
#include "include/core/SkSurface.h"
#include "include/core/SkSwizzle.h"
#include "include/core/SkTextBlob.h"
#include "include/core/SkTextureCompressionType.h"
#include "include/core/SkTiledImageUtils.h"
#include "include/core/SkTypeface.h"
#include "include/core/SkTypes.h"
#include "include/core/SkVertices.h"
// docs/
#include "include/docs/SkPDFDocument.h"
#include "include/docs/SkPDFJpegHelpers.h"

// effects/
#include "include/effects/Sk1DPathEffect.h"
#include "include/effects/Sk2DPathEffect.h"
#include "include/effects/SkBlenders.h"
#include "include/effects/SkColorMatrix.h"
#include "include/effects/SkColorMatrixFilter.h"
#include "include/effects/SkCornerPathEffect.h"
#include "include/effects/SkDashPathEffect.h"
#include "include/effects/SkDiscretePathEffect.h"
#include "include/effects/SkGradient.h"
#include "include/effects/SkHighContrastFilter.h"
#include "include/effects/SkImageFilters.h"
#include "include/effects/SkLumaColorFilter.h"
#include "include/effects/SkOverdrawColorFilter.h"

#include "include/effects/SkRuntimeEffect.h"

#include "include/effects/SkPerlinNoiseShader.h"
#include "include/effects/SkShaderMaskFilter.h"
#include "include/effects/SkTableMaskFilter.h"
#include "include/effects/SkTrimPathEffect.h"

// encode/
#include "include/encode/SkPngEncoder.h"
// #include "include/encode/SkPngRustEncoder.h"
#include "include/encode/SkJpegEncoder.h"

// pathops/
#include "include/pathops/SkPathOps.h"

// svg/
#include "include/svg/SkSVGCanvas.h"

// utils/
#include "include/utils/SkCamera.h"
#include "include/utils/SkCustomTypeface.h"
#include "include/utils/SkNullCanvas.h"
#include "include/utils/SkOrderedFontMgr.h"
#include "include/utils/SkParsePath.h"
#include "include/utils/SkShadowUtils.h"
#include "include/utils/SkTextUtils.h"

extern "C" void C_Bindings_Types(Sink<bool>) {}

//
// codec/SkCodec.h
//

extern "C" SkCodec* C_SkCodec_MakeFromStream(SkStream* stream, const SkCodecs::Decoder* decoders, size_t decodersCount, SkCodec::Result* result, SkCodec::SelectionPolicy selectionPolicy) {
    return SkCodec::MakeFromStream(std::unique_ptr<SkStream>(stream), SkSpan(decoders, decodersCount), result, nullptr, selectionPolicy).release();
}

extern "C" SkCodec* C_SkCodec_MakeFromData(SkData* data) {
    return SkCodec::MakeFromData(sp(data)).release();
}

extern "C" SkCodec* C_SkCodec_MakeFromData2(SkData* data, const SkCodecs::Decoder* decoders, size_t decodersCount) {
    return SkCodec::MakeFromData(sp(data), SkSpan(decoders, decodersCount)).release();
}

extern "C" void C_SkCodec_delete(SkCodec* self) {
    delete self;
}

extern "C" void C_SkCodec_getInfo(const SkCodec* self, SkImageInfo* info) {
    *info = self->getInfo();
}

extern "C" SkISize C_SkCodec_dimensions(const SkCodec* self) {
    return self->dimensions();
}

extern "C" void C_SkCodec_bounds(const SkCodec* self, SkIRect* uninitialized) {
    new (uninitialized) SkIRect(self->bounds());
}

extern "C" SkEncodedOrigin C_SkCodec_getOrigin(const SkCodec* self) {
    return self->getOrigin();
}

extern "C" SkISize C_SkCodec_getScaledDimensions(const SkCodec* self, float desiredScale) {
    return self->getScaledDimensions(desiredScale);
}

extern "C" bool C_SkCodec_hasHighBitDepthEncodedData(const SkCodec* self) {
    return self->hasHighBitDepthEncodedData();
}

extern "C" bool C_SkCodec_getValidSubset(const SkCodec* self, SkIRect* desiredSubset) {
    return self->getValidSubset(desiredSubset);
}

extern "C" SkEncodedImageFormat C_SkCodec_getEncodedFormat(const SkCodec* self) {
    return self->getEncodedFormat();
}

extern "C" SkImage* C_SkCodec_getImage(
    SkCodec *self, const SkImageInfo *info, const SkCodec::Options *opts, SkCodec::Result* result) {

    auto r = self->getImage(*info, opts);
    *result = std::get<1>(r);
    return std::get<0>(r).release();
}

extern "C" SkCodec::Result C_SkCodec_incrementalDecode(SkCodec* self, int* rowsDecoded) {
    return self->incrementalDecode(rowsDecoded);
}

extern "C" SkCodec::SkScanlineOrder C_SkCodec_getScanlineOrder(const SkCodec* self) {
    return self->getScanlineOrder();
}

extern "C" int C_SkCodec_nextScanline(const SkCodec* self) {
    return self->nextScanline();
}

extern "C" int C_SkCodec_getFrameCount(SkCodec* self) {
    return self->getFrameCount();
}

extern "C" void C_SkFrameInfo_Construct(SkCodec::FrameInfo* uninitialized) {
    new (uninitialized) SkCodec::FrameInfo();
}

extern "C" bool C_SkCodec_getFrameInfo(SkCodec* self, int index, SkCodec::FrameInfo* info) {
    return self->getFrameInfo(index, info);
}

extern "C" int C_SkCodec_getRepetitionCount(SkCodec* self) {
    return self->getRepetitionCount();
}

extern "C" SkCodec::IsAnimated C_SkCodec_isAnimated(SkCodec* self) {
    return self->isAnimated();
}

// SkCodecs

extern "C" void C_SkCodecs_Decoder_CopyConstruct(SkCodecs::Decoder* uninitialized, const SkCodecs::Decoder* decoder) {
    new (uninitialized) SkCodecs::Decoder(*decoder);
}

extern "C" const char* C_SkCodecs_Decoder_getId(const SkCodecs::Decoder* decoder, size_t* len) {
    *len = decoder->id.size();
    return decoder->id.data();
}

extern "C" SkCodec* C_SkCodecs_Decoder_MakeFromStream(const SkCodecs::Decoder* decoder, SkStream* stream, SkCodec::Result* result, SkCodecs::DecodeContext context) {
    return decoder->makeFromStream(std::unique_ptr<SkStream>(stream), result, context).release();
}

extern "C" void C_SkCodecs_Decoder_destruct(SkCodecs::Decoder* decoder) {
    decoder->~Decoder();
}

extern "C" SkImage* C_SkCodecs_DeferredImage(SkCodec* codec, const SkAlphaType* alphaType) {
    return SkCodecs::DeferredImage(std::unique_ptr<SkCodec>(codec), alphaType ? std::optional(*alphaType) : std::nullopt).release();
}

//
// codec/*Decoder.h
//

extern "C" void C_SkBmpDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkBmpDecoder::Decoder());
}

extern "C" void C_SkGifDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkGifDecoder::Decoder());
}

extern "C" void C_SkIcoDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkIcoDecoder::Decoder());
}

#if defined(SK_CODEC_DECODES_JPEG)
extern "C" void C_SkJpegDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkJpegDecoder::Decoder());
}
#endif

extern "C" void C_SkPngDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkPngDecoder::Decoder());
}

/*
extern "C" void C_SkPngRustDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkPngRustDecoder::Decoder());
}
*/

extern "C" void C_SkWbmpDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkWbmpDecoder::Decoder());
}

#if defined(SK_CODEC_DECODES_WEBP)

extern "C" void C_SkWebpDecoder_Decoder(SkCodecs::Decoder* uninitialized) {
    new (uninitialized) SkCodecs::Decoder(SkWebpDecoder::Decoder());
}

#endif

//
// codec/SkEncodedOrigin.h
//

extern "C" void C_SkEncodedOriginToMatrix(SkEncodedOrigin origin, int w, int h, SkMatrix* matrix) {
    *matrix = SkEncodedOriginToMatrix(origin, w, h);
}

extern "C" void C_SkEncodedOriginToMatrixInverse(SkEncodedOrigin origin, int w, int h, SkMatrix* matrix) {
    *matrix = SkEncodedOriginToMatrixInverse(origin, w, h);
}

//
// codec/SkPixmapUtils.h
//

extern "C" bool C_SkPixmapUtils_Orient(SkPixmap& dst, const SkPixmap& src, SkEncodedOrigin origin) {
    return SkPixmapUtils::Orient(dst, src, origin);
}

extern "C" void C_SkPixmapUtils_SwapWidthHeight(SkImageInfo* uninitialized, const SkImageInfo& info) {
    new (uninitialized) SkImageInfo(SkPixmapUtils::SwapWidthHeight(info));
}

//
// core/
//

extern "C" void C_Core_Types(
    SkArc *, SkGraphics *, SkCoverageMode *, SkColorChannelFlag *, SkSurfaces::BackendSurfaceAccess) {};

extern "C" void C_SkDocument_Types(SkDocument*) {}

//
// core/SkBlender.h
//

extern "C" SkBlender* C_SkBlender_Mode(SkBlendMode mode) {
    return SkBlender::Mode(mode).release();
}

extern "C" SkBlender* C_SkBlender_Deserialize(const void* data, size_t length) {
    return static_cast<SkBlender*>(SkFlattenable::Deserialize(SkFlattenable::kSkBlender_Type, data, length).release());
}


//
// core/SkColor.h
//

extern "C" uint32_t C_SkColor4f_toBytes_RGBA(const SkColor4f* color) {
    return color->toBytes_RGBA();
}

extern "C" SkColor4f C_SkColor4f_FromBytes_RGBA(uint32_t bytes) {
    return SkColor4f::FromBytes_RGBA(bytes);
}

//
// core/SkCubicMap.h
//

extern "C" SkPoint C_SkCubicMap_computeFromT(const SkCubicMap* self, float t) {
    return self->computeFromT(t);
}

//
// core/SkSurface.h
//

extern "C" SkSurface* C_SkSurfaces_Null(int width, int height) {
    return SkSurfaces::Null(width, height).release();
}

extern "C" SkSurface* C_SkSurfaces_Raster(const SkImageInfo* imageInfo, size_t rowBytes, const SkSurfaceProps* surfaceProps) {
    return SkSurfaces::Raster(*imageInfo, rowBytes, surfaceProps).release();
}

extern "C" SkSurface* C_SkSurfaces_WrapPixels(const SkImageInfo* imageInfo, void* pixels, size_t rowBytes, const SkSurfaceProps* surfaceProps) {
    return SkSurfaces::WrapPixels(*imageInfo, pixels, rowBytes, surfaceProps).release();
}

extern "C" int C_SkSurface_width(const SkSurface* self) {
    return self->width();
}

extern "C" int C_SkSurface_height(const SkSurface* self) {
    return self->height();
}

extern "C" void C_SkSurface_imageInfo(const SkSurface* self, SkImageInfo* info) {
    *info = self->imageInfo();
}

extern "C" SkImage* C_SkSurface_makeImageSnapshot(SkSurface* self, const SkIRect* bounds) {
    if (bounds) {
        return self->makeImageSnapshot(*bounds).release();
    } else {
        return self->makeImageSnapshot().release();
    }
}

extern "C" SkImage* C_SkSurface_makeTemporaryImage(SkSurface* self) {
    return self->makeTemporaryImage().release();
}

extern "C" SkSurface* C_SkSurface_makeSurface(
        SkSurface* self,
        const SkImageInfo* imageInfo) {
    return self->makeSurface(*imageInfo).release();
}

extern "C" SkSurface *C_SkSurface_makeSurface2(
        SkSurface *self,
        int width, int height) {
    return self->makeSurface(width, height).release();
}

extern "C" const SkSurfaceProps* C_SkSurface_props(const SkSurface* self) {
    return &self->props();
}

//
// core/SkImage.h
//

extern "C" SkImage* C_SkImages_RasterFromBitmap(const SkBitmap* bitmap) {
    return SkImages::RasterFromBitmap(*bitmap).release();
}

extern "C" SkImage *C_SkImages_RasterFromCompressedTextureData(SkData *data, int width, int height, SkTextureCompressionType
type) {
    return SkImages::RasterFromCompressedTextureData(sp(data), width, height, type).release();
}

extern "C" SkImage* C_SkImages_DeferredFromEncodedData(SkData* encoded, const SkAlphaType* alphaType) {
    return SkImages::DeferredFromEncodedData(sp(encoded), opt(alphaType)).release();
}

extern "C" SkImage* C_SkImages_DeferredFromGenerator(SkImageGenerator* imageGenerator) {
    return SkImages::DeferredFromGenerator(std::unique_ptr<SkImageGenerator>(imageGenerator)).release();
}

extern "C" SkImage* C_SkImages_DeferredFromPicture(
        SkPicture* picture,
        const SkISize* dimensions,
        const SkMatrix* matrix,
        const SkPaint* paint,
        SkImages::BitDepth bitDepth,
        SkColorSpace* colorSpace,
        const SkSurfaceProps* props) {
    return
        SkImages::DeferredFromPicture(
            sp(picture),
            *dimensions,
            matrix,
            paint,
            bitDepth,
            sp(colorSpace),
            *props
        ).release();
}

extern "C" SkImage* C_SkImages_RasterFromData(const SkImageInfo* info, SkData* pixels, size_t rowBytes) {
    return SkImages::RasterFromData(*info, sp(pixels), rowBytes).release();
}

extern "C" SkImage* C_SkImages_MakeWithFilter(SkImage* image, const SkImageFilter* imageFilter, const SkIRect* subset, const SkIRect* clipBounds, SkIRect* outSubset, SkIPoint* offset) {
    return SkImages::MakeWithFilter(sk_sp(image), imageFilter, *subset, *clipBounds, outSubset, offset).release();
}

extern "C" SkShader* C_SkImage_makeShader(
    const SkImage* self,
    SkTileMode tileMode1, SkTileMode tileMode2,
    const SkSamplingOptions* samplingOptions, const SkMatrix* localMatrix) {
    return self->makeShader(tileMode1, tileMode2, *samplingOptions, localMatrix).release();
}

extern "C" SkShader* C_SkImage_makeRawShader(
    const SkImage* self,
    SkTileMode tileMode1, SkTileMode tileMode2,
    const SkSamplingOptions* samplingOptions, const SkMatrix* localMatrix) {
    return self->makeRawShader(tileMode1, tileMode2, *samplingOptions, localMatrix).release();
}

extern "C" bool C_SkImage_isTextureBacked(const SkImage* self) {
    return self->isTextureBacked();
}

extern "C" size_t C_SkImage_textureSize(const SkImage* self) {
    return self->textureSize();
}

extern "C" bool C_SkImage_isValid(const SkImage* self, SkRecorder* recorder) {
    return self->isValid(recorder);
}

extern "C" SkImage* C_SkImage_makeScaled(const SkImage* self, const SkImageInfo* info, const SkSamplingOptions* sampling) {
    return self->makeScaled(*info, *sampling).release();
}

extern "C" const SkData* C_SkImage_refEncodedData(const SkImage* self) {
    return self->refEncodedData().release();
}

extern "C" SkImage* C_SkImage_makeSubset(const SkImage* self, SkRecorder* recorder, const SkIRect* subset, const SkImage::RequiredProperties* properties) {
    return self->makeSubset(recorder, *subset, *properties).release();
}

extern "C" SkImage* C_SkImage_withDefaultMipmaps(const SkImage* self) {
    return self->withDefaultMipmaps().release();
}

extern "C" SkImage* C_SkImage_makeNonTextureImage(const SkImage* self, GrDirectContext* context) {
    return self->makeNonTextureImage(context).release();
}

extern "C" SkImage* C_SkImage_makeRasterImage(const SkImage* self, GrDirectContext* context, SkImage::CachingHint cachingHint) {
    return self->makeRasterImage(context, cachingHint).release();
}

extern "C" bool C_SkImage_isLazyGenerated(const SkImage* self) {
    return self->isLazyGenerated();
}

extern "C" SkImage* C_SkImage_makeColorSpace(const SkImage* self, SkRecorder* recorder, SkColorSpace* target, const SkImage::RequiredProperties* properties) {
    return self->makeColorSpace(recorder, sp(target), *properties).release();
}

extern "C" SkImage* C_SkImage_reinterpretColorSpace(const SkImage* self, SkColorSpace* newColorSpace) {
    return self->reinterpretColorSpace(sp(newColorSpace)).release();
}

//
// core/SkData.h
//

extern "C" void C_SkData_ref(const SkData* self) {
    self->ref();
}

extern "C" void C_SkData_unref(const SkData* self) {
    self->unref();
}

extern "C" bool C_SkData_unique(const SkData* self) {
    return self->unique();
}

extern "C" SkData* C_SkData_MakeWithCopy(const void* data, size_t length) {
    return SkData::MakeWithCopy(data, length).release();
}

extern "C" SkData* C_SkData_MakeSubset(const SkData* src, size_t offset, size_t length) {
    return SkData::MakeSubset(src, offset, length).release();
}

extern "C" SkData* C_SkData_MakeUninitialized(size_t length) {
    return SkData:: MakeUninitialized(length).release();
}

extern "C" SkData* C_SkData_MakeZeroInitialized(size_t length) {
    return SkData:: MakeZeroInitialized(length).release();
}

extern "C" SkData* C_SkData_MakeWithCString(const char* cstr) {
    return SkData::MakeWithCString(cstr).release();
}

extern "C" SkData* C_SkData_MakeWithoutCopy(const void* data, size_t length) {
    return SkData::MakeWithoutCopy(data, length).release();
}

extern "C" SkData* C_SkData_MakeFromFileName(const char cstr[]) {
    return SkData::MakeFromFileName(cstr).release();
}

extern "C" SkData* C_SkData_MakeFromStream(SkStream* stream, size_t size) {
    return SkData::MakeFromStream(stream, size).release();
}

extern "C" SkData* C_SkData_MakeEmpty() {
    return SkData::MakeEmpty().release();
}

//
// core/SkPaint.h
//

extern "C" void C_SkPaint_destruct(SkPaint* self) {
    self->~SkPaint();
}

extern "C" void C_SkPaint_copy(SkPaint* self, const SkPaint* rhs) {
    *self = *rhs;
}

extern "C" bool C_SkPaint_Equals(const SkPaint* lhs, const SkPaint* rhs) {
    return *lhs == *rhs;
}

extern "C" SkPaint::Style C_SkPaint_getStyle(const SkPaint* self) {
    return self->getStyle();
}

extern "C" uint8_t C_SkPaint_getAlpha(const SkPaint* self) {
    return self->getAlpha();
}

extern "C" SkPaint::Cap C_SkPaint_getStrokeCap(const SkPaint* self) {
    return self->getStrokeCap();
}

extern "C" SkPaint::Join C_SkPaint_getStrokeJoin(const SkPaint* self) {
    return self->getStrokeJoin();
}

extern "C" void C_SkPaint_setShader(SkPaint* self, SkShader* shader) {
    self->setShader(sp(shader));
}

extern "C" void C_SkPaint_setColorFilter(SkPaint* self, SkColorFilter* colorFilter) {
    self->setColorFilter(sp(colorFilter));
}

extern "C" bool C_SkPaint_asBlendMode(const SkPaint* self, SkBlendMode* mode) {
    auto r = self->asBlendMode();
    if (r.has_value()) {
        *mode = *r;
        return true;
    } else {
        return false;
    }
}

extern "C" void C_SkPaint_setBlender(SkPaint* self, SkBlender* blender) {
    self->setBlender(sp(blender));
}

extern "C" void C_SkPaint_setPathEffect(SkPaint* self, SkPathEffect* pathEffect) {
    self->setPathEffect(sp(pathEffect));
}

extern "C" void C_SkPaint_setMaskFilter(SkPaint* self, SkMaskFilter* maskFilter) {
    self->setMaskFilter(sp(maskFilter));
}

extern "C" void C_SkPaint_setImageFilter(SkPaint* self, SkImageFilter* imageFilter) {
    self->setImageFilter(sp(imageFilter));
}

//
// core/SkPathIter.h
//


extern "C" void C_SkPathIter_Construct(
    const SkPoint* points, size_t pointsCount,
    const SkPathVerb* verbs, size_t verbsCount,
    const float* conics, size_t conicsCount,
    SkPathIter* uninitialized) {
    new (uninitialized) SkPathIter(SkSpan(points, pointsCount), SkSpan(verbs, verbsCount), SkSpan(conics, conicsCount));
}

extern "C" void C_SkPathIter_destruct(SkPathIter* self) {
    self->~SkPathIter();
}

extern "C" bool C_SkPathIter_next(SkPathIter* self, SkPathIter::Rec* uninitialized) {
    auto r = self->next();
    if (r.has_value()) {
        new (uninitialized) SkPathIter::Rec(std::move(*r));
        return true;
    }
    return false;
}

extern "C" bool C_SkPathIter_peekNextVerb(SkPathIter* self, SkPathVerb* verb) {
    auto r = self->peekNextVerb();
    if (r.has_value()) {
        *verb = *r;
        return true;
    }
    return false;
}

extern "C" void C_SkPathContourIter_Construct(
    const SkPoint* points, size_t pointsCount,
    const SkPathVerb* verbs, size_t verbsCount,
    const float* conics, size_t conicsCount,
    SkPathContourIter* uninitialized) {
    new (uninitialized) SkPathContourIter(SkSpan(points, pointsCount), SkSpan(verbs, verbsCount), SkSpan(conics, conicsCount));
}

extern "C" void C_SkPathContourIter_destruct(SkPathContourIter* self) {
    self->~SkPathContourIter();
}

extern "C" bool C_SkPathContourIter_next(SkPathContourIter* self, SkPathContourIter::Rec* uninitialized) {
    auto r = self->next();
    if (r.has_value()) {
        new (uninitialized) SkPathContourIter::Rec(std::move(*r));
        return true;
    }
    return false;
}


//
// core/SkPath.h
//

extern "C" void C_SkPath_Construct(SkPath* uninitialized, SkPathFillType fillType) {
    new(uninitialized) SkPath(fillType);
}

extern "C" void C_SkPath_Raw(SkPath* uninitialized, 
    const SkPoint pts[], size_t pointCount,
    const SkPathVerb vbs[], size_t verbCount,
    const SkScalar ws[], size_t wCount,
    SkPathFillType ft, bool isVolatile) {
    new (uninitialized) SkPath(SkPath::Raw(SkSpan(pts, pointCount), SkSpan(vbs, verbCount), SkSpan(ws, wCount), ft, isVolatile));
}

extern "C" void C_SkPath_Make(SkPath* uninitialized, 
    const SkPoint pts[], size_t pointCount,
    const uint8_t vbs[], size_t verbCount,
    const SkScalar ws[], size_t wCount,
    SkPathFillType ft, bool isVolatile) {
    new (uninitialized) SkPath(SkPath::Make(SkSpan(pts, pointCount), SkSpan(vbs, verbCount), SkSpan(ws, wCount), ft, isVolatile));
}

extern "C" void C_SkPath_Rect(SkPath* uninitialized,
    const SkRect& r, SkPathFillType fillType, SkPathDirection dir) {
    new (uninitialized) SkPath(SkPath::Rect(r, fillType, dir));
}

extern "C" void C_SkPath_Oval(SkPath* uninitialized,
    const SkRect& r, SkPathDirection dir) {
    new (uninitialized) SkPath(SkPath::Oval(r, dir));
}

extern "C" void C_SkPath_OvalWithStartIndex(SkPath* uninitialized,
    const SkRect& r, SkPathDirection dir, unsigned int startIndex) {
    new (uninitialized) SkPath(SkPath::Oval(r, dir, startIndex));
}

extern "C" void C_SkPath_Circle(SkPath* uninitialized,
    SkScalar x, SkScalar y, SkScalar r, SkPathDirection dir) {
    new (uninitialized) SkPath(SkPath::Circle(x, y, r, dir));
}

extern "C" void C_SkPath_RRect(SkPath* uninitialized,
    const SkRRect& rr, SkPathDirection dir) {
    new (uninitialized) SkPath(SkPath::RRect(rr, dir));
}

extern "C" void C_SkPath_RRectWithStartIndex(SkPath* uninitialized,
    const SkRRect& r, SkPathDirection dir, unsigned startIndex) {
    new (uninitialized) SkPath(SkPath::RRect(r, dir, startIndex));
}

extern "C" void C_SkPath_Polygon(SkPath* uninitialized,
    const SkPoint pts[], size_t count, bool isClosed,
    SkPathFillType ft,
    bool isVolatile) {
    new (uninitialized) SkPath(SkPath::Polygon(SkSpan(pts, count), isClosed, ft, isVolatile));
}

extern "C" void C_SkPath_destruct(const SkPath* self) {
    self->~SkPath();
}

extern "C" bool C_SkPath_Equals(const SkPath* lhs, const SkPath* rhs) {
    return *lhs == *rhs;
}

extern "C" SkData* C_SkPath_serialize(const SkPath* self) {
    return self->serialize().release();
}

extern "C" bool C_SkPath_ReadFromMemory(SkPath* out, const void* buffer, size_t length) {
    auto path = SkPath::ReadFromMemory(buffer, length);
    if (path) {
        path->swap(*out);
        return true;
    }
    return false;
}

extern "C" void C_SkPath_Iter_destruct(SkPath::Iter* self) {
    self->~Iter();
}

extern "C" bool C_SkPath_Iter_isCloseLine(const SkPath::Iter* self) {
    return self->isCloseLine();
}

extern "C" void C_SkPath_RawIter_Construct(SkPath::RawIter* uninitialized) {
    new(uninitialized)SkPath::RawIter();
}

extern "C" void C_SkPath_RawIter_destruct(SkPath::RawIter* self) {
    self->~RawIter();
}

extern "C" SkPath::Verb C_SkPath_RawIter_peek(const SkPath::RawIter* self) {
    return self->peek();
}

extern "C" SkPathFillType C_SkPath_getFillType(const SkPath* self) {
    return self->getFillType();
}

extern "C" void C_SkPath_makeFillType(const SkPath* self, SkPathFillType newFillType, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->makeFillType(newFillType));
}

extern "C" void C_SkPath_makeToggleInverseFillType(const SkPath* self, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->makeToggleInverseFillType());
}

extern "C" void C_SkPath_makeIsVolatile(const SkPath* self, bool isVolatile, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->makeIsVolatile(isVolatile));
}

extern "C" const SkPoint* C_SkPath_points(const SkPath* self, size_t* count) {
    auto span = self->points();
    *count = span.size();
    return span.begin();
}

extern "C" const SkPathVerb* C_SkPath_verbs(const SkPath* self, size_t* count) {
    auto span = self->verbs();
    *count = span.size();
    return span.begin();
}

extern "C" const float* C_SkPath_conicWeights(const SkPath* self, size_t* count) {
    auto span = self->conicWeights();
    *count = span.size();
    return span.begin();
}

extern "C" size_t C_SkPath_getPoints(const SkPath* self, SkPoint* points, size_t count) {
    return self->getPoints(SkSpan(points, count));
}

extern "C" size_t C_SkPath_getVerbs(const SkPath* self, uint8_t* verbs, size_t count) {
    return self->getVerbs(SkSpan(verbs, count));
}

extern "C" void C_SkPath_iter(const SkPath* self, SkPathIter* uninitialized) {
    new (uninitialized) SkPathIter(self->iter());
}

extern "C" const SkRect* C_SkPath_getBounds(const SkPath* self) {
    return &self->getBounds();
}

extern "C" void C_SkPath_computeTightBounds(const SkPath* self, SkRect* uninitialized) {
    new (uninitialized) SkRect(self->computeTightBounds());
}

extern "C" bool C_SkPath_tryMakeTransform(const SkPath* self, const SkMatrix* matrix, SkPath* uninitialized) {
    if (auto res = self->tryMakeTransform(*matrix)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" bool C_SkPath_tryMakeOffset(const SkPath* self, float dx, float dy, SkPath* uninitialized) {
    if (auto res = self->tryMakeOffset(dx, dy)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" bool C_SkPath_tryMakeScale(const SkPath* self, float sx, float sy, SkPath* uninitialized) {
    if (auto res = self->tryMakeScale(sx, sy)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" void C_SkPath_makeOffset(const SkPath* self, float dx, float dy, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->makeOffset(dx, dy));
}

extern "C" void C_SkPath_makeTransform(const SkPath* self, const SkMatrix* matrix, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->makeTransform(*matrix));
}

extern "C" bool C_SkPath_getLastPt(const SkPath* self, SkPoint* point) {
    if (auto last = self->getLastPt()) {
        *point = *last;
        return true;
    }
    return false;
}

//
// core/SkPathBuilder.h
//

extern "C" SkPathBuilder* C_SkPathBuilder_new() {
    return new SkPathBuilder();
}

extern "C" SkPathBuilder* C_SkPathBuilder_newWithFillType(SkPathFillType fillType) {
    return new SkPathBuilder(fillType);
}

extern "C" SkPathBuilder* C_SkPathBuilder_newFromPath(const SkPath& path) {
     return new SkPathBuilder(path);
}

extern "C" bool C_SkPathBuilder_computeFiniteBounds(const SkPathBuilder* self, SkRect* rect) {
    if (auto bounds = self->computeFiniteBounds()) {
        *rect = *bounds;
        return true;
    }
    return false;
}

extern "C" bool C_SkPathBuilder_computeTightBounds(const SkPathBuilder* self, SkRect* rect) {
    if (auto bounds = self->computeTightBounds()) {
        *rect = *bounds;
        return true;
    }
    return false;
}

extern "C" SkPathBuilder* C_SkPathBuilder_clone(const SkPathBuilder* pathBuilder) {
    return new SkPathBuilder(*pathBuilder);
}

extern "C" void C_SkPathBuilder_delete(SkPathBuilder* self) {
    delete self;
}

extern "C" bool C_SkPathBuilder_equals(const SkPathBuilder* self, const SkPathBuilder* other) {
    return *self == *other;
}

extern "C" void C_SkPathBuilder_snapshot(const SkPathBuilder* self, const SkMatrix* mx, SkPath* out) {
    self->snapshot(mx).swap(*out);
}

extern "C" void C_SkPathBuilder_detach(SkPathBuilder* self, const SkMatrix* mx, SkPath* out) {
    self->detach(mx).swap(*out);
}

extern "C" void C_SkPathBuilder_polylineTo(SkPathBuilder* self, const SkPoint* points, size_t count) {
    self->polylineTo(SkSpan(points, count));
}

extern "C" void C_SkPathBuilder_addPolygon(SkPathBuilder* self, const SkPoint* points, size_t count, bool close) {
    self->addPolygon(SkSpan(points, count), close);
}

extern "C" bool C_SkPathBuilder_isEmpty(const SkPathBuilder* self) {
    return self->isEmpty();
}

extern "C" bool C_SkPathBuilder_getLastPt(const SkPathBuilder* self, SkPoint* point) {
    if (auto last = self->getLastPt()) {
        *point = *last;
        return true;
    }
    return false;
}

extern "C" void C_SkPathBuilder_setPoint(SkPathBuilder* self, size_t index, SkPoint p) {
    self->setPoint(index, p);
}

extern "C" size_t C_SkPathBuilder_countPoints(const SkPathBuilder* self) {
    return self->countPoints();
}

extern "C" const SkPoint* C_SkPathBuilder_points(const SkPathBuilder* self, size_t* count) {
    auto span = self->points();
    *count = span.size();
    return span.begin();
}

extern "C" const SkPathVerb* C_SkPathBuilder_verbs(const SkPathBuilder* self, size_t* count) {
    auto span = self->verbs();
    *count = span.size();
    return span.begin();
}

extern "C" const float* C_SkPathBuilder_conicWeights(const SkPathBuilder* self, size_t* count) {
    auto span = self->conicWeights();
    *count = span.size();
    return span.begin();
}

extern "C" void C_SkPathBuilder_dumpToString(const SkPathBuilder* self, SkPathBuilder::DumpFormat format, SkString* str) {
    *str = self->dumpToString(format);
}

extern "C" void C_SkPathBuilder_dump(const SkPathBuilder* self, SkPathBuilder::DumpFormat format) {
    self->dump(format);
}

//
// SkPathMeasure
//

extern "C" void C_SkPathMeasure_destruct(const SkPathMeasure* self) {
    self->~SkPathMeasure();
}

//
// core/SkPathTypes.h
//

extern "C" void
C_SkPathTypes_Types(SkPathFillType *, SkPathDirection *, SkPathSegmentMask *, SkPathVerb *) {}

//
// core/SkPathUtils.h
//

extern "C" bool C_PathUtils_FillPathWithPaint(const SkPath* src, const SkPaint* paint, SkPathBuilder* dst, const SkRect* cullRect, const SkMatrix* matrix) {
    return skpathutils::FillPathWithPaint(*src, *paint, dst, cullRect, *matrix);
}

//
// core/SkCanvas.h
// Note: bindgen layout is broken, so we are forced to allocate Canvas instances on the heap only.
//

extern "C" void C_SkCanvas_SaveLayerRec_Construct(SkCanvas::SaveLayerRec* uninitialized) {
    new (uninitialized) SkCanvas::SaveLayerRec();
}

extern "C" void C_SkCanvas_SaveLayerRec_destruct(SkCanvas::SaveLayerRec* self) {
    self->~SaveLayerRec();
}

extern "C" SkCanvas* C_SkCanvas_newEmpty() {
    return new SkCanvas();
}

extern "C" SkCanvas* C_SkCanvas_newWidthHeightAndProps(int width, int height, const SkSurfaceProps* props) {
    return new SkCanvas(width, height, props);
}

extern "C" SkCanvas* C_SkCanvas_newFromBitmap(const SkBitmap* bitmap) {
    return new SkCanvas(*bitmap);
}

extern "C" SkCanvas* C_SkCanvas_newFromBitmapAndProps(const SkBitmap* bitmap, const SkSurfaceProps* props) {
    return new SkCanvas(*bitmap, *props);
}

extern "C" void C_SkCanvas_delete(const SkCanvas* self) {
    delete self;
}

extern "C" SkCanvas* C_SkCanvas_MakeRasterDirect(const SkImageInfo* info, void* pixels, size_t row_bytes, const SkSurfaceProps* props) {
    return SkCanvas::MakeRasterDirect(*info, pixels, row_bytes, props).release();
}

extern "C" void C_SkCanvas_imageInfo(const SkCanvas* self, SkImageInfo* info) {
    *info = self->imageInfo();
}

extern "C" void C_SkCanvas_getBaseLayerSize(const SkCanvas* self, SkISize* size) {
    *size = self->getBaseLayerSize();
}

extern "C" SkSurface* C_SkCanvas_makeSurface(SkCanvas* self, const SkImageInfo* info, const SkSurfaceProps* props) {
    return self->makeSurface(*info, props).release();
}

extern "C" void C_SkCanvas_clipShader(SkCanvas* self, SkShader* shader, SkClipOp op) {
    self->clipShader(sp(shader), op);
}

extern "C" void C_SkCanvas_getLocalClipBounds(const SkCanvas* self, SkRect* uninitialized) {
    new (uninitialized) SkRect(self->getLocalClipBounds());
}

extern "C" void C_SkCanvas_getDeviceClipBounds(const SkCanvas* self, SkIRect* result) {
    *result = self->getDeviceClipBounds();
}

extern "C" bool C_SkCanvas_isClipEmpty(const SkCanvas* self) {
    return self->isClipEmpty();
}

extern "C" bool C_SkCanvas_isClipRect(const SkCanvas* self) {
    return self->isClipRect();
}

extern "C" void C_SkCanvas_getLocalToDevice(const SkCanvas* self, SkM44* uninitialized) {
    new(uninitialized) SkM44(self->getLocalToDevice());
}

extern "C" void C_SkCanvas_getTotalMatrix(const SkCanvas* self, SkMatrix* matrix) {
    *matrix = self->getTotalMatrix();
}

extern "C" void C_SkCanvas_discard(SkCanvas* self) {
    self->discard();
}

extern "C" void C_SkCanvas_drawPoints(SkCanvas* self, SkCanvas::PointMode mode, const SkPoint* points, size_t pointCount, const SkPaint* paint) {
    self->drawPoints(mode, SkSpan(points, pointCount), *paint);
}

extern "C" void C_SkCanvas_drawGlyphs(
    SkCanvas* self, 
    const SkGlyphID* glyphs, size_t glyphCount,
    const SkPoint* positions,
    SkPoint origin, const SkFont* font, const SkPaint* paint) {
    self->drawGlyphs(
        SkSpan(glyphs, glyphCount), 
        SkSpan(positions, glyphCount),
        origin, *font, *paint);
}

extern "C" void C_SkCanvas_drawGlyphs2(
    SkCanvas* self, 
    const SkGlyphID* glyphs, size_t glyphCount,
    const SkPoint* positions,
    const uint32_t* clusters,
    const char* utf8Text, size_t textSize,
    SkPoint origin, const SkFont* font, const SkPaint* paint) {
    self->drawGlyphs(
        SkSpan(glyphs, glyphCount), 
        SkSpan(positions, glyphCount),
        SkSpan(clusters, glyphCount),
        SkSpan(utf8Text, textSize), 
        origin, *font, *paint);
}

extern "C" void C_SkCanvas_drawGlyphsRSXform(
    SkCanvas* self, 
    const SkGlyphID* glyphs, size_t glyphCount,
    const SkRSXform* xforms,
    SkPoint origin, const SkFont* font, const SkPaint* paint) {
    self->drawGlyphsRSXform(
        SkSpan(glyphs, glyphCount), 
        SkSpan(xforms, glyphCount),
        origin, *font, *paint);
}

extern "C" void C_SkCanvas_drawAtlas(
    SkCanvas* self,
    const SkImage* atlas, 
    const SkRSXform* xform, size_t xformCount,
    const SkRect* tex, const SkColor* colors, SkBlendMode mode,
    const SkSamplingOptions* sampling, const SkRect* cullRect, const SkPaint* paint) {

    self->drawAtlas(
        atlas,
        SkSpan(xform, xformCount),
        SkSpan(tex, xformCount),
        SkSpan(colors, colors == nullptr ? 0 : xformCount),
        mode, *sampling, cullRect, paint);
}

//
// core/SkAutoCanvasRestore.h
//

extern "C" void C_SkAutoCanvasRestore_Construct(SkAutoCanvasRestore* uninitialized, SkCanvas* canvas, bool doSave) {
    new (uninitialized) SkAutoCanvasRestore(canvas, doSave);
}

extern "C" void C_SkAutoCanvasRestore_destruct(const SkAutoCanvasRestore* self) {
    self->~SkAutoCanvasRestore();
}

extern "C" void C_SkAutoCanvasRestore_restore(SkAutoCanvasRestore* self) {
    self->restore();
}

//
// core/SkImageInfo.h
//

extern "C" void C_SkColorInfo_Construct(SkColorInfo* uninitialized) {
    new (uninitialized) SkColorInfo();
}

extern "C" void C_SkColorInfo_Construct2(SkColorInfo* uninitialized, SkColorType ct, SkAlphaType at, SkColorSpace* cs) {
    new (uninitialized) SkColorInfo(ct, at, sp(cs));
}

extern "C" void C_SkColorInfo_destruct(SkColorInfo* self) {
    self->~SkColorInfo();
}

extern "C" void C_SkColorInfo_Copy(const SkColorInfo* from, SkColorInfo* to) {
    *to = *from;
}

extern "C" bool C_SkColorInfo_Equals(const SkColorInfo* lhs, const SkColorInfo* rhs) {
    return *lhs == *rhs;
}


extern "C" void C_SkColorInfo_makeAlphaType(const SkColorInfo* self, SkAlphaType newAlphaType, SkColorInfo* uninitialized) {
    new (uninitialized) SkColorInfo(self->makeAlphaType(newAlphaType));
}

extern "C" void C_SkColorInfo_makeColorType(const SkColorInfo* self, SkColorType newColorType, SkColorInfo* uninitialized) {
    new (uninitialized) SkColorInfo(self->makeColorType(newColorType));
}

extern "C" void C_SkColorInfo_makeColorSpace(const SkColorInfo* self, SkColorSpace* newColorSpace, SkColorInfo* uninitialized) {
    new (uninitialized) SkColorInfo(self->makeColorSpace(sp(newColorSpace)));
}

extern "C" void C_SkImageInfo_Construct(SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo();
}

extern "C" void C_SkImageInfo_destruct(SkImageInfo* self) {
    self->~SkImageInfo();
}

extern "C" void C_SkImageInfo_Copy(const SkImageInfo* from, SkImageInfo* to) {
    *to = *from;
}

extern "C" bool C_SkImageInfo_Equals(const SkImageInfo* lhs, const SkImageInfo* rhs) {
    return *lhs == *rhs;
}

extern "C" void C_SkImageInfo_Make(int width, int height, SkColorType ct, SkAlphaType at, SkColorSpace* cs, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::Make(width, height, ct, at, sp(cs)));
}

extern "C" void C_SkImageInfo_MakeN32(int width, int height, SkAlphaType at, SkColorSpace* cs, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::MakeN32(width, height, at, sp(cs)));
}

extern "C" void C_SkImageInfo_MakeS32(int width, int height, SkAlphaType at, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::MakeS32(width, height, at));
}

extern "C" void C_SkImageInfo_MakeN32Premul(int width, int height, SkColorSpace* cs, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::MakeN32Premul(width, height, sp(cs)));
}

extern "C" void C_SkImageInfo_MakeA8(int width, int height, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::MakeA8(width, height));
}

extern "C" void C_SkImageInfo_MakeUnknown(int width, int height, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(SkImageInfo::MakeUnknown(width, height));
}

extern "C" void C_SkImageInfo_makeColorSpace(const SkImageInfo* self, SkColorSpace* cs, SkImageInfo* uninitialized) {
    new (uninitialized) SkImageInfo(self->makeColorSpace(sp(cs)));
}

extern "C" void C_SkImageInfo_reset(SkImageInfo* self) {
    self->reset();
}

//
// core/SkColorSpace.h
//

extern "C" void C_SkColorSpace_Types(SkColorSpacePrimaries *) {}

extern "C" void C_SkColorSpace_ref(const SkColorSpace* self) {
    self->ref();
}

extern "C" void C_SkColorSpace_unref(const SkColorSpace* self) {
    self->unref();
}

extern "C" bool C_SkColorSpace_unique(const SkColorSpace* self) {
    return self->unique();
}

extern "C" SkColorSpace* C_SkColorSpace_MakeICC(const void* buffer, size_t size) {
    skcms_ICCProfile profile;
    if (skcms_Parse(buffer, size, &profile)) {
        return SkColorSpace::Make(profile).release();
    }
    return nullptr;
}

extern "C" SkColorSpace* C_SkColorSpace_MakeSRGB() {
    return SkColorSpace::MakeSRGB().release();
}

extern "C" SkColorSpace* C_SkColorSpace_MakeSRGBLinear() {
    return SkColorSpace::MakeSRGBLinear().release();
}

extern "C" SkColorSpace* C_SkColorSpace_MakeCICP(
    SkNamedPrimaries::CicpId colorPrimaries,
    SkNamedTransferFn::CicpId transferCharacteristics) {
    return SkColorSpace::MakeCICP(colorPrimaries, transferCharacteristics).release();
}

extern "C" SkColorSpace* C_SkColorSpace_makeLinearGamma(const SkColorSpace* self) {
    return self->makeLinearGamma().release();
}

extern "C" SkColorSpace* C_SkColorSpace_makeSRGBGamma(const SkColorSpace* self) {
    return self->makeSRGBGamma().release();
}

extern "C" SkColorSpace* C_SkColorSpace_makeColorSpin(const SkColorSpace* self) {
    return self->makeColorSpin().release();
}

extern "C" SkData* C_SkColorSpace_serialize(const SkColorSpace* self) {
    return self->serialize().release();
}

extern "C" SkColorSpace* C_SkColorSpace_Deserialize(const void* data, size_t length) {
    return SkColorSpace::Deserialize(data, length).release();
}

extern "C" uint32_t C_SkColorSpace_transferFnHash(const SkColorSpace* self) {
    return self->transferFnHash();
}

extern "C" uint64_t C_SkColorSpace_hash(const SkColorSpace* self) {
    return self->hash();
}

//
// SkM44
//

extern "C" void C_SkM44_Types(SkV2 *) {};

extern "C" bool C_SkM44_equals(const SkM44 *self, const SkM44 *other) {
    return *self == *other;
}

extern "C" void C_SkM44_RectToRect(const SkRect* src, const SkRect* dst, SkM44* uninitialized) {
    new (uninitialized) SkM44(SkM44::RectToRect(*src, *dst));
}

extern "C" void C_SkM44_LookAt(const SkV3* eye, const SkV3* center, const SkV3* up, SkM44* uninitialized) {
    new (uninitialized) SkM44(SkM44::LookAt(*eye, *center, *up));
}

extern "C" void C_SkM44_Perspective(float near, float far, float angle, SkM44* uninitialized) {
    new (uninitialized) SkM44(SkM44::Perspective(near, far, angle));
}

extern "C" void C_SkM44_transpose(const SkM44* self, SkM44* uninitialized) {
    new (uninitialized) SkM44(self->transpose());
}

extern "C" SkV4 C_SkM44_map(const SkM44* self, float x, float y, float z, float w) {
    return self->map(x, y, z, w);
}

//
// core/SkMatrix.h
//

extern "C" void C_SkMatrix_ScaleTranslate(SkScalar sx, SkScalar sy, SkScalar tx, SkScalar ty, SkMatrix* uninitialized) {
    new (uninitialized) SkMatrix(SkMatrix::ScaleTranslate(sx, sy, tx, ty));
}

extern "C" bool C_SkMatrix_Equals(const SkMatrix* self, const SkMatrix* rhs) {
    return *self == *rhs;
}

extern "C" SkScalar* C_SkMatrix_SubscriptMut(SkMatrix* self, size_t index) {
    return &((*self)[static_cast<int>(index)]);
}

extern "C" SkMatrix::TypeMask C_SkMatrix_getType(const SkMatrix* self) {
    return self->getType();
}

extern "C" bool C_SkMatrix_rectStaysRect(const SkMatrix* self) {
    return self->rectStaysRect();
}

extern "C" bool C_SkMatrix_hasPerspective(const SkMatrix* self) {
    return self->hasPerspective();
}

extern "C" bool C_SkMatrix_Rect2Rect(const SkRect* src, const SkRect* dst, SkMatrix::ScaleToFit scaleToFit, SkMatrix* m) {
    auto r = SkMatrix::Rect2Rect(*src, *dst, scaleToFit);
    if (r.has_value()) {
        *m = *r;
        return true;
    }
    return false;
}

extern "C" bool C_SkMatrix_setPolyToPoly(SkMatrix* self, const SkPoint* src, size_t srcLen, const SkPoint* dst, size_t dstLen) {
    return self->setPolyToPoly(SkSpan(src, srcLen), SkSpan(dst, dstLen));
}

extern "C" bool C_SkMatrix_invert(const SkMatrix* self, SkMatrix* inverse) {
    return self->invert(inverse);
}

extern "C" bool C_SkMatrix_isFinite(const SkMatrix* self) {
    return self->isFinite();
}

extern "C" const SkMatrix* C_SkMatrix_InvalidMatrix() {
    return &SkMatrix::InvalidMatrix();
}

extern "C" void C_SkMatrix_normalizePerspective(SkMatrix* self) {
    self->normalizePerspective();
}

extern "C" void C_SkMatrix_mapPoints(const SkMatrix* self, SkPoint* dst, const SkPoint* src, size_t count) {
    self->mapPoints(SkSpan(dst, count), SkSpan(src, count));
}

extern "C" void C_SkMatrix_mapHomogeneousPoints(const SkMatrix* self, SkPoint3* dst, const SkPoint3* src, size_t count) {
    self->mapHomogeneousPoints(SkSpan(dst, count), SkSpan(src, count));
}

extern "C" void C_SkMatrix_mapPointsToHomogeneous(const SkMatrix* self, SkPoint3* dst, const SkPoint* src, size_t count) {
    self->mapPointsToHomogeneous(SkSpan(dst, count), SkSpan(src, count));
}

extern "C" SkPoint C_SkMatrix_mapPoint(const SkMatrix* self, SkPoint p) {
    return self->mapPoint(p);
}

extern "C" SkPoint C_SkMatrix_mapPointAffine(const SkMatrix* self, SkPoint p) {
    return self->mapPointAffine(p);
}

extern "C" void C_SkMatrix_mapVectors(const SkMatrix* self, SkVector* dst, const SkVector* src, size_t count) {
    self->mapVectors(SkSpan(dst, count), SkSpan(src, count));
}

//
// SkSurfaceProps
//

extern "C" bool C_SkSurfaceProps_Equals(const SkSurfaceProps* self, const SkSurfaceProps* rhs) {
    return *self == *rhs;
}

//
// SkBitmap
//

extern "C" void C_SkBitmap_Construct(SkBitmap* uninitialized) {
    new (uninitialized) SkBitmap();
}

extern "C" void C_SkBitmap_destruct(SkBitmap* self) {
    self->~SkBitmap();
}

extern "C" void C_SkBitmap_Copy(const SkBitmap* from, SkBitmap* to) {
    *to = *from;
}

extern "C" bool C_SkBitmap_ComputeIsOpaque(const SkBitmap* self) {
    return SkBitmap::ComputeIsOpaque(*self);
}

extern "C" void C_SkBitmap_setColorSpace(SkBitmap* self, SkColorSpace* colorSpace) {
    self->setColorSpace(sp(colorSpace));
}

extern "C" bool C_SkBitmap_tryAllocN32Pixels(SkBitmap* self, int width, int height, bool isOpaque) {
    return self->tryAllocN32Pixels(width, height, isOpaque);
}

extern "C" bool C_SkBitmap_tryAllocPixels(SkBitmap* self) {
    return self->tryAllocPixels();
}

extern "C" SkIPoint C_SkBitmap_pixelRefOrigin(const SkBitmap* self) {
    return self->pixelRefOrigin();
}

extern "C" void C_SkBitmap_setPixelRef(SkBitmap* self, SkPixelRef* pixelRef, int dx, int dy) {
    self->setPixelRef(sp(pixelRef), dx, dy);
}

extern "C" bool C_SkBitmap_readyToDraw(const SkBitmap* self) {
    return self->readyToDraw();
}

extern "C" void C_SkBitmap_eraseARGB(const SkBitmap* self, U8CPU a, U8CPU r, U8CPU g, U8CPU b) {
    self->eraseARGB(a, r, g, b);
}

extern "C" float C_SkBitmap_getAlphaf(const SkBitmap* self, int x, int y) {
    return self->getAlphaf(x, y);
}

extern "C" bool C_SkBitmap_extractAlpha(const SkBitmap* self, SkBitmap* dst, const SkPaint* paint, SkIPoint* offset) {
    return self->extractAlpha(dst, paint, offset);
}

extern "C" SkShader* C_SkBitmap_makeShader(
    const SkBitmap* self,
    SkTileMode tmx, SkTileMode tmy,
    const SkSamplingOptions* sampling,
    const SkMatrix* localMatrix) {
    return self->makeShader(tmx, tmy, *sampling, localMatrix).release();
}

extern "C" SkImage* C_SkBitmap_asImage(const SkBitmap* self) {
    return self->asImage().release();
}

//
// core/SkPicture.h
//

extern "C" SkPicture* C_SkPicture_MakeFromData(const SkData* data) {
    return SkPicture::MakeFromData(data).release();
}

extern "C" SkPicture* C_SkPicture_MakeFromData2(const void* data, size_t size) {
    return SkPicture::MakeFromData(data, size).release();
}

extern "C" SkData* C_SkPicture_serialize(const SkPicture* self) {
    return self->serialize().release();
}

extern "C" SkPicture* C_SkPicture_MakePlaceholder(const SkRect& cull) {
    return SkPicture::MakePlaceholder(cull).release();
}

extern "C" void C_SkPicture_playback(const SkPicture* self, SkCanvas* canvas) {
    self->playback(canvas);
}

extern "C" void C_SkPicture_cullRect(const SkPicture* self, SkRect* uninitialized) {
    new (uninitialized) SkRect(self->cullRect());
}

extern "C" uint32_t C_SkPicture_uniqueID(const SkPicture* self) {
    return self->uniqueID();
}

extern "C" int C_SkPicture_approximateOpCount(const SkPicture* self, bool nested) {
    return self->approximateOpCount(nested);
}

// note: returning size_t produces a linker error.
extern "C" void C_SkPicture_approximateBytesUsed(const SkPicture* self, size_t* out) {
    *out = self->approximateBytesUsed();
}

extern "C" SkShader *C_SkPicture_makeShader(
    const SkPicture *self,
    SkTileMode tmx, SkTileMode tmy,
    SkFilterMode mode,
    const SkMatrix *localMatrix, const SkRect *tileRect)
{
    return self->makeShader(tmx, tmy, mode, localMatrix, tileRect).release();
}

//
// core/SkRRect.h
//

extern "C" void C_SkRRect_Construct(SkRRect* uninitialized) {
    new (uninitialized) SkRRect();
}

extern "C" SkRRect::Type C_SkRRect_getType(const SkRRect* self) {
    return self->getType();
}

extern "C" void C_SkRRect_setRect(SkRRect* self, const SkRect* rect) {
    self->setRect(*rect);
}

extern "C" void C_SkRRect_dumpToString(const SkRRect* self, bool asHex, SkString* str) {
    *str = self->dumpToString(asHex);
}

extern "C" bool C_SkRRect_Equals(const SkRRect* lhs, const SkRRect* rhs) {
    return *lhs == *rhs;
}

//
// SkRegion
//

extern "C" void C_SkRegion_destruct(SkRegion* region) {
    region->~SkRegion();
}

extern "C" bool C_SkRegion_Equals(const SkRegion* lhs, const SkRegion* rhs) {
    return *lhs == *rhs;
}

extern "C" bool C_SkRegion_set(SkRegion* self, const SkRegion* region) {
    return self->set(*region);
}

extern "C" bool C_SkRegion_quickContains(const SkRegion* self, const SkIRect* r) {
    return self->quickContains(*r);
}

extern "C" void C_SkRegion_getBoundaryPath(const SkRegion* self, SkPath* uninitialized) {
    new (uninitialized) SkPath(self->getBoundaryPath());
}

extern "C" void C_SkRegion_Iterator_Construct(SkRegion::Iterator* uninitialized) {
    new (uninitialized) SkRegion::Iterator();
}

extern "C" void C_SkRegion_Iterator_destruct(SkRegion::Iterator* self) {
    self->~Iterator();
}

extern "C" const SkRegion* C_SkRegion_Iterator_rgn(const SkRegion::Iterator* self) {
    return self->rgn();
}

extern "C" void C_SkRegion_Cliperator_destruct(SkRegion::Cliperator* self) {
    self->~Cliperator();
}

extern "C" void C_SkRegion_Spanerator_destruct(SkRegion::Spanerator* self) {
    self->~Spanerator();
}

//
// SkFontStyle
//

extern "C" void C_SkFontStyle_Construct(SkFontStyle* uninitialized) {
    new (uninitialized) SkFontStyle();
}

extern "C" void C_SkFontStyle_Construct2(SkFontStyle* uninitialized, int weight, int width, SkFontStyle::Slant slant) {
    new (uninitialized) SkFontStyle(weight, width, slant);
}

extern "C" bool C_SkFontStyle_Equals(const SkFontStyle* lhs, const SkFontStyle* rhs) {
    return *lhs == *rhs;
}

extern "C" int C_SkFontStyle_weight(const SkFontStyle* self) {
    return self->weight();
}

extern "C" int C_SkFontStyle_width(const SkFontStyle* self) {
    return self->width();
}

extern "C" SkFontStyle::Slant C_SkFontStyle_slant(const SkFontStyle* self) {
    return self->slant();
}

//
// SkTextBlob
//

extern "C" void C_SkTextBlob_ref(const SkTextBlob* self) {
    self->ref();
}

extern "C" void C_SkTextBlob_unref(const SkTextBlob* self) {
    self->unref();
}

extern "C" bool C_SkTextBlob_unique(const SkTextBlob* self) {
    return self->unique();
}

extern "C" SkTextBlob* C_SkTextBlob_MakeFromText(const void* text, size_t byteLength, const SkFont* font, SkTextEncoding encoding) {
    return SkTextBlob::MakeFromText(text, byteLength, *font, encoding).release();
}

extern "C" SkTextBlob *C_SkTextBlob_MakeFromPosTextH(const void *text, size_t byteLength,
                                                     const SkScalar xPos[], size_t characterCount, 
                                                     SkScalar constY, const SkFont *font,
                                                     SkTextEncoding encoding) {
    return SkTextBlob::MakeFromPosTextH(text, byteLength, SkSpan(xPos, characterCount), constY, *font, encoding).release();
}

extern "C" SkTextBlob *C_SkTextBlob_MakeFromPosText(const void *text, size_t byteLength,
                                                    const SkPoint pos[], size_t characterCount,
                                                    const SkFont *font,
                                                    SkTextEncoding encoding) {
    return SkTextBlob::MakeFromPosText(text, byteLength, SkSpan(pos, characterCount), *font, encoding).release();
}

extern "C" SkTextBlob *C_SkTextBlob_MakeFromRSXform(const void *text, size_t byteLength,
                                                    const SkRSXform xform[], size_t characterCount,
                                                    const SkFont *font,
                                                    SkTextEncoding encoding) {
    return SkTextBlob::MakeFromRSXform(text, byteLength, SkSpan(xform, characterCount), *font, encoding).release();
}

extern "C" void C_SkTextBlob_Iter_destruct(SkTextBlob::Iter* self) {
    self->~Iter();
}

extern "C" void C_SkTextBlobBuilder_destruct(SkTextBlobBuilder* self) {
    self->~SkTextBlobBuilder();
}

extern "C" SkTextBlob* C_SkTextBlobBuilder_make(SkTextBlobBuilder* self) {
    return self->make().release();
}

//
// core/SkTypeface.h
//

extern "C" void C_SkTypeface_fontStyle(const SkTypeface* self, SkFontStyle* uninitialized) {
    new (uninitialized) SkFontStyle(self->fontStyle());
}

extern "C" int C_SkTypeface_getVariationDesignPosition(const SkTypeface* self, SkFontArguments::VariationPosition::Coordinate* coordinates, size_t count) {
    return self->getVariationDesignPosition(SkSpan(coordinates, count));
}

extern "C" int C_SkTypeface_getVariationDesignParameters(const SkTypeface* self, SkFontParameters::Variation::Axis* parameters, size_t count) {
    return self->getVariationDesignParameters(SkSpan(parameters, count));
}

extern "C" SkTypeface* C_SkTypeface_makeClone(const SkTypeface* self, const SkFontArguments* arguments) {
    return self->makeClone(*arguments).release();
}

extern "C" SkData* C_SkTypeface_serialize(const SkTypeface* self, SkTypeface::SerializeBehavior behavior) {
    return self->serialize(behavior).release();
}

extern "C" void C_SkTypeface_serialize2(const SkTypeface* self, SkWStream* stream, SkTypeface::SerializeBehavior behavior) {
    self->serialize(stream, behavior);
}

extern "C" SkTypeface* C_SkTypeface_MakeDeserialize(SkStream* stream, SkFontMgr* lastResortFontMgr) {
    return SkTypeface::MakeDeserialize(stream, sp(lastResortFontMgr)).release();
}

extern "C" void C_SkTypeface_unicharsToGlyphs(const SkTypeface* self, const SkUnichar* uni, size_t uniCount, SkGlyphID* glyphs, size_t glyphsCount) {
    self->unicharsToGlyphs(SkSpan(uni, uniCount), SkSpan(glyphs, glyphsCount));
}

extern "C" size_t C_SkTypeface_textToGlyphs(const SkTypeface* self, const void* text, size_t byteLength, SkTextEncoding encoding, SkGlyphID* glyphs, size_t glyphsCount) {
    return self->textToGlyphs(text, byteLength, encoding, SkSpan(glyphs, glyphsCount));
}

extern "C" int C_SkTypeface_readTableTags(const SkTypeface* self, SkFontTableTag* tags, size_t count) {
    return self->readTableTags(SkSpan(tags, count));
}

extern "C" SkData* C_SkTypeface_copyTableData(const SkTypeface* self, SkFontTableTag tag) {
    return self->copyTableData(tag).release();
}

extern "C" bool C_SkTypeface_getKerningPairAdjustments(const SkTypeface* self, const SkGlyphID* glyphs, size_t glyphsCount, int32_t* adjustments, size_t adjustmentsCount) {
    return self->getKerningPairAdjustments(SkSpan(glyphs, glyphsCount), SkSpan(adjustments, adjustmentsCount));
}

extern "C" SkStreamAsset* C_SkTypeface_openStream(const SkTypeface* self, int* ttcIndex) {
    return self->openStream(ttcIndex).release();
}

extern "C" void C_SkTypeface_getBounds(const SkTypeface* self, SkRect* uninitialized) {
    new (uninitialized) SkRect(self->getBounds());
}

extern "C" void C_SkTypeface_LocalizedStrings_unref(SkTypeface::LocalizedStrings* self) {
    self->unref();
}

extern "C" bool C_SkTypeface_LocalizedStrings_next(SkTypeface::LocalizedStrings* self, SkString* string, SkString* language) {
    auto ls = SkTypeface::LocalizedString();
    if (self->next(&ls)) {
        *string = ls.fString;
        *language = ls.fLanguage;
        return true;
    }
    return false;
}

//
// core/SkFlattenable.h
//

extern "C" const char* C_SkFlattenable_getTypeName(const SkFlattenable* self) {
    return self->getTypeName();
}

extern "C" SkData* C_SkFlattenable_serialize(const SkFlattenable* self) {
    return self->serialize().release();
}

//
// core/SkFont.h
//

extern "C" void C_SkFont_ConstructFromTypeface(SkFont* uninitialized, SkTypeface* typeface_) {
    new (uninitialized) SkFont(sp(typeface_));
}

extern "C" void C_SkFont_ConstructFromTypefaceWithSize(SkFont* uninitialized, SkTypeface* typeface_, SkScalar size) {
    new (uninitialized) SkFont(sp(typeface_), size);
}

extern "C" void C_SkFont_ConstructFromTypefaceWithSizeScaleAndSkew(SkFont* uninitialized, SkTypeface* typeface_, SkScalar size, SkScalar scaleX, SkScalar skewX) {
    new (uninitialized) SkFont(sp(typeface_), size, scaleX, skewX);
}

extern "C" void C_SkFont_CopyConstruct(SkFont* uninitialized, const SkFont* font) {
    new (uninitialized) SkFont(*font);
}

extern "C" void C_SkFont_destruct(SkFont* self) {
    self->~SkFont();
}

extern "C" bool C_SkFont_Equals(const SkFont* self, const SkFont* other) {
    return *self == *other;
}

extern "C" SkFont::Edging C_SkFont_getEdging(const SkFont* self) {
    return self->getEdging();
}

extern "C" SkFontHinting C_SkFont_getHinting(const SkFont* self) {
    return self->getHinting();
}

extern "C" void C_SkFont_makeWithSize(const SkFont* self, SkScalar size, SkFont* result) {
    *result = self->makeWithSize(size);
}

extern "C" SkTypeface* C_SkFont_getTypeface(SkFont* self) {
    return self->getTypeface();
}

extern "C" void C_SkFont_setTypeface(SkFont* self, SkTypeface* tf) {
    self->setTypeface(sp(tf));
}

extern "C" size_t C_SkFont_textToGlyphs(
    const SkFont* self,
    const void* text, size_t byteLength, 
    SkTextEncoding encoding,
    SkGlyphID* glyphs, size_t glyphsCount) {
    return self->textToGlyphs(text, byteLength, encoding, SkSpan(glyphs, glyphsCount));
}

extern "C" void C_SkFont_unicharsToGlyphs(
    const SkFont* self,
    const SkUnichar* src, size_t charsCount,
    SkGlyphID* dst) {
    self->unicharsToGlyphs(SkSpan(src, charsCount), SkSpan(dst, charsCount));
}

extern "C" void C_SkFont_getWidthBounds(
    const SkFont* self,
    const SkGlyphID* glyphs, size_t glyphCount,
    SkScalar* widths, SkRect* bounds,
    const SkPaint* paint) {
    self->getWidthsBounds(
        SkSpan(glyphs, glyphCount),
        SkSpan(widths, widths ? glyphCount : 0),
        SkSpan(bounds, bounds ? glyphCount : 0), paint);
}

extern "C" void C_SkFont_getPos(
    const SkFont* self,
    const SkGlyphID* glyphs, size_t glyphCount,
    SkPoint* pos,
    SkPoint origin) {
    self->getPos(SkSpan(glyphs, glyphCount), SkSpan(pos, glyphCount), origin);
}

extern "C" void C_SkFont_getXPos(
    const SkFont* self,
    const SkGlyphID* glyphs, size_t glyphCount,
    SkScalar* xPos,
    SkScalar origin) {
    self->getXPos(SkSpan(glyphs, glyphCount), SkSpan(xPos, glyphCount), origin);
}

extern "C" void C_SkFont_getIntercepts(
    const SkFont* self, 
    const SkGlyphID* glyphs, 
    size_t count, 
    const SkPoint* pos, 
    SkScalar top, SkScalar bottom, 
    const SkPaint* paint, 
    VecSink<SkScalar>* vs) {
    auto r = self->getIntercepts(SkSpan(glyphs, count), SkSpan(pos, count), top, bottom, paint);
    vs->set(r);
}

extern "C" bool C_SkFont_getPath(const SkFont* self, SkGlyphID glyphID, SkPath* pathR) {
    auto path = self->getPath(glyphID);
    if (!path) {
        return false;
    }
    *pathR = path.value();
    return true;
}

//
// core/SkFontArguments.h
//

extern "C" void C_SkFontArguments_construct(SkFontArguments* uninitialized) {
    new (uninitialized) SkFontArguments();
}

extern "C" void C_SkFontArguments_destruct(SkFontArguments* self) {
    self->~SkFontArguments();
}

extern "C" void C_SkFontArguments_setCollectionIndex(SkFontArguments* self, int collectionIndex) {
    self->setCollectionIndex(collectionIndex);
}

extern "C" void C_SkFontArguments_setVariationDesignPosition(SkFontArguments* self, SkFontArguments::VariationPosition position) {
    self->setVariationDesignPosition(position);
}

extern "C" void C_SkFontArguments_setPalette(SkFontArguments* self, SkFontArguments::Palette palette) {
    self->setPalette(palette);
}

extern "C" SkFontArguments::VariationPosition
C_SkFontArguments_getVariationDesignPosition(const SkFontArguments *self) {
    return self->getVariationDesignPosition();
}

extern "C" SkFontArguments::Palette
C_SkFontArguments_getPalette(const SkFontArguments *self) {
    return self->getPalette();
}

extern "C" void C_SkFontArguments_setSyntheticBold(SkFontArguments* self, int syntheticBold) {
    self->setSyntheticBold(
        syntheticBold < 0 ? std::nullopt : std::optional<bool>(syntheticBold != 0));
}

extern "C" int C_SkFontArguments_getSyntheticBold(const SkFontArguments* self) {
    auto syntheticBold = self->getSyntheticBold();
    if (!syntheticBold.has_value()) {
        return -1;
    }
    return syntheticBold.value() ? 1 : 0;
}

extern "C" void C_SkFontArguments_setSyntheticOblique(SkFontArguments* self,
                                                        int syntheticOblique) {
    self->setSyntheticOblique(
        syntheticOblique < 0 ? std::nullopt : std::optional<bool>(syntheticOblique != 0));
}

extern "C" int C_SkFontArguments_getSyntheticOblique(const SkFontArguments* self) {
    auto syntheticOblique = self->getSyntheticOblique();
    if (!syntheticOblique.has_value()) {
        return -1;
    }
    return syntheticOblique.value() ? 1 : 0;
}

//
// core/SkFontMgr.h
//

extern "C" int C_SkFontStyleSet_count(SkFontStyleSet* self) {
    return self->count();
}

extern "C" void C_SkFontStyleSet_getStyle(SkFontStyleSet* self, int index, SkFontStyle* fontStyle, SkString* style) {
    self->getStyle(index, fontStyle, style);
}

extern "C" SkTypeface* C_SkFontStyleSet_createTypeface(SkFontStyleSet* self, int index) {
    return self->createTypeface(index).release();
}

extern "C" SkTypeface* C_SkFontStyleSet_matchStyle(SkFontStyleSet* self, const SkFontStyle* pattern) {
    return self->matchStyle(*pattern).release();
}

extern "C" SkFontStyleSet* C_SkFontStyleSet_CreateEmpty() {
    return SkFontStyleSet::CreateEmpty().release();
}


extern "C" SkFontStyleSet* C_SkFontMgr_createStyleSet(const SkFontMgr* self, int index) {
    return self->createStyleSet(index).release();
}

extern "C" SkFontStyleSet* C_SkFontMgr_matchFamily(const SkFontMgr* self, const char familyName[]) {
    return self->matchFamily(familyName).release();
}

extern "C" SkTypeface* C_SkFontMgr_matchFamilyStyle(
    const SkFontMgr* self,
    const char familyName[],
    const SkFontStyle* style) {
    return self->matchFamilyStyle(familyName, *style).release();
}

extern "C" SkTypeface* C_SkFontMgr_matchFamilyStyleCharacter(
    const SkFontMgr* self,
    const char familyName[], const SkFontStyle* style,
    const char* bcp47[], int bcp47Count,
    SkUnichar character) {
    return self->matchFamilyStyleCharacter(familyName, *style, bcp47, bcp47Count, character).release();
}

extern "C" SkTypeface* C_SkFontMgr_makeFromData(const SkFontMgr* self, SkData* data, int ttcIndex) {
    return self->makeFromData(sp(data), ttcIndex).release();
}

// note: this function _consumes_ / deletes the stream.
extern "C" SkTypeface* C_SkFontMgr_makeFromStream(const SkFontMgr* self, SkStreamAsset* stream, int ttcIndex) {
    return self->makeFromStream(std::unique_ptr<SkStreamAsset>(stream), ttcIndex).release();
}

extern "C" SkTypeface* C_SkFontMgr_legacyMakeTypeface(const SkFontMgr* self, const char familyName[], SkFontStyle style) {
    return self->legacyMakeTypeface(familyName, style).release();
}

// From skia/tools/FontToolUtils.cpp

#if defined(SK_BUILD_FOR_WIN) && (defined(SK_FONTMGR_GDI_AVAILABLE) || defined(SK_FONTMGR_DIRECTWRITE_AVAILABLE))
#include "include/ports/SkTypeface_win.h"
#endif

#if defined(SK_BUILD_FOR_ANDROID) && defined(SK_FONTMGR_ANDROID_AVAILABLE)
#include "include/ports/SkFontMgr_android.h"
#include "include/ports/SkFontScanner_FreeType.h"
#endif

#if defined(SK_FONTMGR_CORETEXT_AVAILABLE) && (defined(SK_BUILD_FOR_IOS) || defined(SK_BUILD_FOR_MAC))
#include "include/ports/SkFontMgr_mac_ct.h"
#endif

#if defined(SK_FONTMGR_FONTCONFIG_AVAILABLE) && defined(SK_TYPEFACE_FACTORY_FREETYPE)
#include "include/ports/SkFontMgr_fontconfig.h"
#include "include/ports/SkFontScanner_FreeType.h"
#endif

#if defined(SK_FONTMGR_FREETYPE_DIRECTORY_AVAILABLE)
#include "include/ports/SkFontMgr_directory.h"
#endif

#if defined(SK_FONTMGR_FREETYPE_EMPTY_AVAILABLE)
#include "include/ports/SkFontMgr_empty.h"
#endif

#ifndef SK_FONT_FILE_PREFIX
#  if defined(SK_BUILD_FOR_MAC) || defined(SK_BUILD_FOR_IOS)
#    define SK_FONT_FILE_PREFIX "/System/Library/Fonts/"
#  else
#    define SK_FONT_FILE_PREFIX "/usr/share/fonts/"
#  endif
#endif

/// Creates a new system font manager, empty if none is available.
extern "C" SkFontMgr* C_SkFontMgr_NewSystem() {
    sk_sp<SkFontMgr> mgr;
// Prefer DirectWrite over GDI, see <https://github.com/rust-skia/rust-skia/issues/948>
#if defined(SK_BUILD_FOR_WIN) && defined(SK_FONTMGR_DIRECTWRITE_AVAILABLE)
    mgr = SkFontMgr_New_DirectWrite();
#elif defined(SK_BUILD_FOR_WIN) && defined(SK_FONTMGR_GDI_AVAILABLE)
    mgr = SkFontMgr_New_GDI();
#elif defined(SK_BUILD_FOR_ANDROID) && defined(SK_FONTMGR_ANDROID_AVAILABLE) && defined(SK_TYPEFACE_FACTORY_FREETYPE)
    mgr = SkFontMgr_New_Android(nullptr, SkFontScanner_Make_FreeType());
#elif defined(SK_FONTMGR_CORETEXT_AVAILABLE) && (defined(SK_BUILD_FOR_IOS) || defined(SK_BUILD_FOR_MAC))
    mgr = SkFontMgr_New_CoreText(nullptr);
#elif defined(SK_FONTMGR_FONTCONFIG_AVAILABLE) && defined(SK_TYPEFACE_FACTORY_FREETYPE)
    mgr = SkFontMgr_New_FontConfig(nullptr, SkFontScanner_Make_FreeType());
#elif defined(SK_FONTMGR_FREETYPE_DIRECTORY_AVAILABLE)
    // In particular, this is used on ChromeOS, which is Linux-like but doesn't have
    // FontConfig.
    mgr = SkFontMgr_New_Custom_Directory(SK_FONT_FILE_PREFIX);
#elif defined(SK_FONTMGR_FREETYPE_EMPTY_AVAILABLE)
    mgr = SkFontMgr_New_Custom_Empty();
#else
    mgr = SkFontMgr::RefEmpty();
#endif
    return mgr.release();
}

extern "C" SkFontMgr* C_SkFontMgr_RefEmpty() {
    return SkFontMgr::RefEmpty().release();
}

extern "C" SkFontMgr* C_SkFontMgr_NewCustomEmpty() {
#if defined(SK_FONTMGR_FREETYPE_EMPTY_AVAILABLE)
    return SkFontMgr_New_Custom_Empty().release();
#else
    return nullptr;
#endif
}

//
// core/SkFontParameters.h
//

extern "C" bool C_SkFontParameters_Variation_Axis_isHidden(const SkFontParameters::Variation::Axis* self) {
    return self->isHidden();
}

extern "C" void C_SkFontParameters_Variation_Axis_setHidden(SkFontParameters::Variation::Axis* self, bool hidden) {
    self->setHidden(hidden);
}

//
// SkVertices
//

extern "C" void C_SkVertices_ref(const SkVertices* self) {
    self->ref();
}

extern "C" void C_SkVertices_unref(const SkVertices* self) {
    self->unref();
}

extern "C" bool C_SkVertices_unique(const SkVertices* self) {
    return self->unique();
}

extern "C" SkVertices* C_SkVertices_MakeCopy(
    SkVertices::VertexMode mode, int vertexCount,
    const SkPoint positions[],
    const SkPoint texs[],
    const SkColor colors[],
    int indexCount,
    const uint16_t indices[]) {
    return SkVertices::MakeCopy(mode, vertexCount, positions, texs, colors, indexCount, indices).release();
}

//
// SkVertices::Builder
//

extern "C" void C_SkVertices_Builder_destruct(SkVertices::Builder* builder) {
    builder->~Builder();
}

extern "C" SkVertices* C_SkVertices_Builder_detach(SkVertices::Builder* builder) {
    return builder->detach().release();
}

//
// SkPictureRecorder
//

extern "C" void C_SkPictureRecorder_Construct(SkPictureRecorder *uninitialized) {
    new (uninitialized) SkPictureRecorder();
}

extern "C" void C_SkPictureRecorder_destruct(SkPictureRecorder *self) {
    self->~SkPictureRecorder();
}

extern "C" SkCanvas* C_SkPictureRecorder_beginRecording(SkPictureRecorder *self, const SkRect* bounds, bool useBBH) {
    std::unique_ptr<SkBBHFactory> factory;
    if (useBBH) factory = std::make_unique<SkRTreeFactory>();
    return self->beginRecording(*bounds, factory.get());
}

extern "C" SkPicture* C_SkPictureRecorder_finishRecordingAsPicture(SkPictureRecorder* self, const SkRect* cullRect) {
    if (cullRect){
        return self->finishRecordingAsPictureWithCull(*cullRect).release();
    } else {
        return self->finishRecordingAsPicture().release();
    }
}

extern "C" SkDrawable* C_SkPictureRecorder_finishRecordingAsDrawable(SkPictureRecorder* self) {
    return self->finishRecordingAsDrawable().release();
}

//
// core/SkPixelRef.h
//

extern "C" int C_SkPixelRef_width(const SkPixelRef* self) {
    return self->width();
}

extern "C" int C_SkPixelRef_height(const SkPixelRef* self) {
    return self->height();
}

extern "C" void* C_SkPixelRef_pixels(const SkPixelRef* self) {
    return self->pixels();
}

extern "C" size_t C_SkPixelRef_rowBytes(const SkPixelRef* self) {
    return self->rowBytes();
}

extern "C" bool C_SkPixelRef_isImmutable(const SkPixelRef* self) {
    return self->isImmutable();
}

extern "C" void C_SkPixelRef_notifyAddedToCache(SkPixelRef* self) {
    self->notifyAddedToCache();
}

//
// core/SkPoint.h
//

extern "C" bool C_SkPoint_isFinite(const SkPoint* self) {
    return self->isFinite();
}

//
// core/SkRecorder.h
//

extern "C" void C_SkRecorder_delete(SkRecorder* self) {
    delete self;
}

extern "C" void C_SkRecorder_type(const SkRecorder* self, SkRecorder::Type* ty) {
    *ty = self->type();
}

extern "C" skcpu::Recorder* C_SkRecorder_cpuRecorder(SkRecorder* self) {
    return self->cpuRecorder();
}

//
// core/SkRect.h
//

extern "C" bool C_SkIRect_isEmpty(const SkIRect* self) {
    return self->isEmpty();
}

extern "C" bool C_SkIRect_contains(const SkIRect* self, const SkRect* rect) {
    return self->contains(*rect);
}

extern "C" bool C_SkRect_Bounds(const SkPoint* pts, size_t count, SkRect* bounds) {
    auto boundsOpt = SkRect::Bounds(SkSpan(pts, count));
    auto hasValue = boundsOpt.has_value();
    if (hasValue) {
        *bounds = *boundsOpt;
    }
    return hasValue;
}

extern "C" bool C_SkRect_setBoundsCheck(SkRect* self, const SkPoint* pts, size_t count) {
    return self->setBoundsCheck(SkSpan(pts, count));
}

extern "C" void C_SkRect_setBoundsNoCheck(SkRect* self, const SkPoint* pts, size_t count) {
    self->setBoundsNoCheck(SkSpan(pts, count));
}

extern "C" void C_SkRect_round(const SkRect* self, SkIRect* dst) {
    self->round(dst);
}

extern "C" void C_SkRect_roundIn(const SkRect* self, SkIRect* dst) {
    self->roundIn(dst);
}

extern "C" void C_SkRect_dumpToString(const SkRect* self, bool asHex, SkString* str) {
    *str = self->dumpToString(asHex);
}

extern "C" void C_SkRect_roundOut(const SkRect* self, SkIRect* dst) {
    self->roundOut(dst);
}

//
// core/SkRefCntBase.h
//

extern "C" void C_SkRefCntBase_ref(const SkRefCntBase* self) {
    self->ref();
}

extern "C" void C_SkRefCntBase_unref(const SkRefCntBase* self) {
    self->unref();
}

extern "C" bool C_SkRefCntBase_unique(const SkRefCntBase* self) {
    return self->unique();
}

//
// SkColorFilter
//

extern "C" SkColorFilter* C_SkColorFilter_makeComposed(const SkColorFilter* self, SkColorFilter* inner) {
    return self->makeComposed(sp(inner)).release();
}

extern "C" SkColorFilter* C_SkColorFilter_withWorkingColorSpace(const SkColorFilter* self, SkColorSpace* colorSpace) {
    return self->makeWithWorkingColorSpace(sp(colorSpace)).release();
}

extern "C" SkColorFilter* C_SkColorFilter_Deserialize(const void* data, size_t length) {
    return SkColorFilter::Deserialize(data, length).release();
}

extern "C" SkColor4f C_SkColorFilter_filterColor4f(
    const SkColorFilter* self, 
    const SkColor4f* srcColor, 
    SkColorSpace* srcCS, 
    SkColorSpace* dstCS) {
    return self->filterColor4f(*srcColor, srcCS, dstCS);
}

//
// SkColorFilters
//

extern "C" SkColorFilter* C_SkColorFilters_Compose(SkColorFilter* outer, SkColorFilter* inner) {
    return SkColorFilters::Compose(sp(outer), sp(inner)).release();
}

extern "C" SkColorFilter* C_SkColorFilters_Blend2(const SkColor4f* c, SkColorSpace* colorSpace, SkBlendMode blendMode) {
    return SkColorFilters::Blend(*c, sp(colorSpace), blendMode).release();
}

extern "C" SkColorFilter* C_SkColorFilters_Blend(const SkColor c, SkBlendMode blendMode) {
    return SkColorFilters::Blend(c, blendMode).release();
}


extern "C" SkColorFilter* C_SkColorFilters_Matrix(const SkColorMatrix* colorMatrix, SkColorFilters::Clamp clamp) {
    return SkColorFilters::Matrix(*colorMatrix, clamp).release();
}

extern "C" SkColorFilter* C_SkColorFilters_MatrixRowMajor(const SkScalar array[20], SkColorFilters::Clamp clamp) {
    return SkColorFilters::Matrix(array, clamp).release();
}

extern "C" SkColorFilter* C_SkColorFilters_HSLAMatrixOfColorMatrix(const SkColorMatrix& colorMatrix) {
    return SkColorFilters::HSLAMatrix(colorMatrix).release();
}

extern "C" SkColorFilter* C_SkColorFilters_HSLAMatrix(const float rowMajor[20]) {
    return SkColorFilters::HSLAMatrix(rowMajor).release();
}

extern "C" SkColorFilter* C_SkColorFilters_LinearToSRGBGamma() {
    return SkColorFilters::LinearToSRGBGamma().release();
}

extern "C" SkColorFilter* C_SkColorFilters_SRGBToLinearGamma() {
    return SkColorFilters::SRGBToLinearGamma().release();
}

extern "C" SkColorFilter* C_SkColorFilters_Lerp(float t, SkColorFilter* dst, SkColorFilter* src) {
    return SkColorFilters::Lerp(t, sp(dst), sp(src)).release();
}

extern "C" SkColorFilter* C_SkColorFilters_Table(const uint8_t table[256]) {
    return SkColorFilters::Table(table).release();
}

extern "C" SkColorFilter* C_SkColorFilters_TableARGB(const uint8_t tableA[256], const uint8_t tableR[256], const uint8_t tableG[256], const uint8_t tableB[256]) {
    return SkColorFilters::TableARGB(tableA, tableR, tableG, tableB).release();
}

extern "C" SkColorFilter* C_SkColorFilters_Table2(SkColorTable* table) {
    return SkColorFilters::Table(sp(table)).release();
}

extern "C" SkColorFilter* C_SkColorFilters_Lighting(SkColor mul, SkColor add) {
    return SkColorFilters::Lighting(mul, add).release();
}

//
// SkContourMeasureIter
//

extern "C" void C_SkContourMeasureIter_destruct(SkContourMeasureIter* self) {
    self->~SkContourMeasureIter();
}

extern "C" SkContourMeasure* C_SkContourMeasureIter_next(SkContourMeasureIter* self) {
    return self->next().release();
}

extern "C" SkScalar C_SkContourMeasure_length(const SkContourMeasure* self) {
    return self->length();
}

extern "C" bool C_SkContourMeasure_isClosed(const SkContourMeasure* self) {
    return self->isClosed();
}

extern "C" void C_SkContourMeasure_begin(const SkContourMeasure* self, SkContourMeasure::ForwardVerbIterator* uninitialized) {
    new (uninitialized) SkContourMeasure::ForwardVerbIterator(self->begin());
}

extern "C" void C_SkContourMeasure_end(const SkContourMeasure* self, SkContourMeasure::ForwardVerbIterator* uninitialized) {
    new (uninitialized) SkContourMeasure::ForwardVerbIterator(self->end());
}

extern "C" bool C_SkContourMeasure_ForwardVerbIterator_Equals(const SkContourMeasure::ForwardVerbIterator* a, const SkContourMeasure::ForwardVerbIterator* b) {
    return *a == *b;
}

extern "C" void C_SkContourMeasure_ForwardVerbIterator_item(const SkContourMeasure::ForwardVerbIterator* self, SkContourMeasure::VerbMeasure* uninitialized) {
    new (uninitialized) SkContourMeasure::VerbMeasure(**self);
}

extern "C" void C_SkContourMeasure_ForwardVerbIterator_next(SkContourMeasure::ForwardVerbIterator* self) {
    ++*self;
}

//
// core/SkDataTable.h
//

extern "C" int C_SkDataTable_count(const SkDataTable* self) {
    return self->count();
}

extern "C" SkDataTable *C_SkDataTable_MakeEmpty() {
    return SkDataTable::MakeEmpty().release();
}

extern "C" SkDataTable *C_SkDataTable_MakeCopyArrays(const void *const *ptrs,
                                                     const size_t *sizes, int count) {
    return SkDataTable::MakeCopyArrays(ptrs, sizes, count).release();
}

extern "C" SkDataTable *C_SkDataTable_MakeCopyArray(const void *array, size_t elemSize, int count) {
    return SkDataTable::MakeCopyArray(array, elemSize, count).release();
}

//
// core/SkColorTable.h
//

extern "C" SkColorTable* C_SkColorTable_Make(const uint8_t tableA[256], const uint8_t tableR[256], const uint8_t tableG[256], const uint8_t tableB[256]) {
    return SkColorTable::Make(tableA, tableR, tableG, tableB).release();
}

extern "C" const uint8_t* C_SkColorTable_getTable(const SkColorTable* self, size_t index) {
    switch (index) {
        case 0: return self->alphaTable();
        case 1: return self->redTable();
        case 2: return self->greenTable();
        case 3: return self->blueTable();
        default: return nullptr;
    }
}

//
// core/SkDrawable.h
//

extern "C" SkDrawable* C_SkDrawable_Deserialize(const void* data, size_t length) {
    return SkDrawable::Deserialize(data, length).release();
}

extern "C" SkPicture* C_SkDrawable_makePictureSnapshot(SkDrawable* self) {
    return self->makePictureSnapshot().release();
}

extern "C" void C_SkDrawable_getBounds(SkDrawable* self, SkRect* result) {
    *result = self->getBounds();
}

//
// SkImageFilter
//

extern "C" void C_SkImageFilter_computeFastBounds(const SkImageFilter* self, const SkRect* bounds, SkRect* uninitialized) {
    new (uninitialized) SkRect(self->computeFastBounds(*bounds));
}

extern "C" SkImageFilter* C_SkImageFilter_makeWithLocalMatrix(const SkImageFilter* self, const SkMatrix* matrix) {
    return self->makeWithLocalMatrix(*matrix).release();
}

extern "C" SkImageFilter* C_SkImageFilter_Deserialize(const void* data, size_t length) {
    return SkImageFilter::Deserialize(data, length).release();
}

extern "C" void C_SkImageFilter_filterBounds(
    const SkImageFilter* self,
    const SkIRect* src,
    const SkMatrix* ctm,
    SkImageFilter::MapDirection mapDirection, 
    const SkIRect* inputRect,
    SkIRect *result) {
    *result = self ->filterBounds(*src, *ctm, mapDirection, inputRect);
}

extern "C" bool C_SkImageFilter_isColorFilterNode(const SkImageFilter* self, SkColorFilter** filterPtr) {
    return self->isColorFilterNode(filterPtr);
}

extern "C" int C_SkImageFilter_countInputs(const SkImageFilter* self) {
    return self->countInputs();
}

extern "C" const SkImageFilter* C_SkImageFilter_getInput(const SkImageFilter* self, int i) {
    return self->getInput(i);
}

//
// core/SkImageGenerator.h
//

extern "C" void C_SkImageGenerator_delete(SkImageGenerator *self) {
    delete self;
}

extern "C" const SkData *C_SkImageGenerator_refEncodedData(SkImageGenerator *self) {
    return self->refEncodedData().release();
}

extern "C" bool C_SkImageGenerator_isValid(const SkImageGenerator* self, SkRecorder* recorder) {
    return self->isValid(recorder);
}

extern "C" bool C_SkImageGenerator_isProtected(const SkImageGenerator* self) {
    return self->isProtected();
}

extern "C" bool C_SkImageGenerator_isTextureGenerator(const SkImageGenerator *self) {
    return self->isTextureGenerator();
}

//
// core/SkString.h
//

extern "C" void C_SkString_destruct(SkString* self) {
    self->~SkString();
}

extern "C" void C_SkString_CopyConstruct(SkString* uninitialized, const SkString* str) {
    new (uninitialized) SkString(*str);
}

extern "C" const char* C_SkString_c_str_size(const SkString* self, size_t* size) {
    *size = self->size();
    return self->c_str();
}

extern "C" {
    void C_SkStrings_construct(SkStrings *uninitialized, SkString *string, size_t count) {
        new (uninitialized) SkStrings{
                std::vector<SkString>(std::make_move_iterator(string), std::make_move_iterator(string + count))
        };
    }

    void C_SkStrings_destruct(SkStrings* self) {
        self->~SkStrings();
    }
    
    const SkString* C_SkStrings_ptr_count(const SkStrings* self, size_t* count) {
        *count = self->strings.size();
        return &self->strings.front();
    }
}

// `std::string_view` interop (since m105).

extern "C" const char* C_string_view_ptr_size(const std::string_view* self, size_t* size) {
    *size = self->size();
    return *size ? self->data() : nullptr;
}

// and for completeness `std::string`

extern "C" const char* C_string_ptr_size(const std::string* self, size_t* size) {
    *size = self->size();
    return *size ? self->data() : nullptr;
}

//
// core/SkStrokeRec.h
//

extern "C" void C_SkStrokeRec_destruct(SkStrokeRec* self) {
    self->~SkStrokeRec();
}

extern "C" void C_SkStrokeRec_copy(const SkStrokeRec* self, SkStrokeRec* other) {
    *other = *self;
}

extern "C" SkPaint::Cap C_SkStrokeRec_getCap(const SkStrokeRec* self) {
    return self->getCap();
}

extern "C" SkPaint::Join C_SkStrokeRec_getJoin(const SkStrokeRec* self) {
    return self->getJoin();
}

extern "C" bool C_SkStrokeRec_hasEqualEffect(const SkStrokeRec* self, const SkStrokeRec* other) {
    return self->hasEqualEffect(*other);
}

//
// SkPathEffect
//

extern "C" SkPathEffect* C_SkPathEffect_MakeSum(SkPathEffect* first, SkPathEffect* second) {
    return SkPathEffect::MakeSum(sp(first), sp(second)).release();
}

extern "C" SkPathEffect* C_SkPathEffect_MakeCompose(SkPathEffect* outer, SkPathEffect* inner) {
    return SkPathEffect::MakeCompose(sp(outer), sp(inner)).release();
}

extern "C" SkPathEffect* C_SkPathEffect_Deserialize(const void* data, size_t length) {
    return SkPathEffect::Deserialize(data, length).release();
}

//
// SkPixmap
//

extern "C" void C_SkPixmap_destruct(SkPixmap* self) {
    self->~SkPixmap();
}

extern "C" void C_SkPixmap_setColorSpace(SkPixmap* self, SkColorSpace* colorSpace) {
    self->setColorSpace(sp(colorSpace));
}

//
// SkMaskFilter
//

extern "C" SkMaskFilter* C_SkMaskFilter_MakeBlur(SkBlurStyle style, SkScalar sigma, bool respectCTM) {
    return SkMaskFilter::MakeBlur(style, sigma, respectCTM).release();
}

extern "C" SkMaskFilter* C_SkMaskFilter_Deserialize(const void* data, size_t length) {
    return SkMaskFilter::Deserialize(data, length).release();
}

//
// core/SkSize.h
//

extern "C" SkISize C_SkSize_toRound(const SkSize* size) {
    return size->toRound();
}

extern "C" SkISize C_SkSize_toCeil(const SkSize* size) {
    return size->toCeil();
}

extern "C" SkISize C_SkSize_toFloor(const SkSize* size) {
    return size->toFloor();
}

//
// core/SkShader.h
//

extern "C" bool C_SkShader_isOpaque(const SkShader* self) {
    return self->isOpaque();
}

extern "C" bool C_SkShader_isAImage(const SkShader* self) {
    return self->isAImage();
}

extern "C" SkShader* C_SkShader_makeWithLocalMatrix(const SkShader* self, const SkMatrix* matrix) {
    return self->makeWithLocalMatrix(*matrix).release();
}

extern "C" SkShader* C_SkShader_makeWithColorFilter(const SkShader* self, SkColorFilter* colorFilter) {
    return self->makeWithColorFilter(sp(colorFilter)).release();
}

extern "C" SkShader* C_SkShader_makeWithWorkingColorSpace(const SkShader* self, SkColorSpace* inputCS, SkColorSpace* outputCS) {
    return self->makeWithWorkingColorSpace(sp(inputCS), sp(outputCS)).release();
}

extern "C" SkShader* C_SkShaders_Empty() {
    return SkShaders::Empty().release();
}

extern "C" SkShader* C_SkShaders_Color(SkColor color) {
    return SkShaders::Color(color).release();
}

extern "C" SkShader* C_SkShaders_Color2(const SkColor4f* color, SkColorSpace* colorSpace) {
    return SkShaders::Color(*color, sp(colorSpace)).release();
}

extern "C" SkShader* C_SkShaders_Blend(SkBlender* blender, SkShader* dst, SkShader* src) {
    return SkShaders::Blend(sp(blender), sp(dst), sp(src)).release();
}

extern "C" SkShader* C_SkShaders_CoordClamp(SkShader* shader, const SkRect* subset) {
    return SkShaders::CoordClamp(sp(shader), *subset).release();
}

extern "C" SkShader* C_SkShaders_Image(SkImage* image, SkTileMode tmx, SkTileMode tmy, const SkSamplingOptions& options, const SkMatrix* localMatrix) {
    return SkShaders::Image(sp(image), tmx, tmy, options, localMatrix).release();
}

extern "C" SkShader* C_SkShaders_RawImage(SkImage* image, SkTileMode tmx, SkTileMode tmy, const SkSamplingOptions& options, const SkMatrix* localMatrix) {
    return SkShaders::RawImage(sp(image), tmx, tmy, options, localMatrix).release();
}

extern "C" SkShader* C_SkShader_Deserialize(const void* data, size_t length) {
    // note: dynamic_cast may lead to a linker error here on iOS x86_64
    // https://github.com/rust-skia/rust-skia/issues/146
    // "typeinfo for SkShader", referenced from:
    //      _C_SkShader_Deserialize in libcanvasnative.a(bindings.o)
    return (SkShader*)(SkShader::Deserialize(SkFlattenable::Type::kSkShader_Type, data, length).release());
}

//
// core/SkStream.h
//

extern "C" void C_SkStream_delete(SkStream* stream) {
    delete stream;
}

extern "C" size_t C_SkStream_read(SkStream* stream, void* buffer, size_t len) {
    return stream->read(buffer, len);
}

extern "C" size_t C_SkStreamAsset_getLength(const SkStreamAsset* self) {
    return self->getLength();
}

extern "C" void C_SkWStream_destruct(SkWStream* self) {
    self->~SkWStream();
}

extern "C" bool C_SkWStream_write(SkWStream* self, const void* buffer, size_t size) {
    return self->write(buffer, size);
}

extern "C" SkMemoryStream* C_SkMemoryStream_MakeDirect(const void* data, size_t length) {
    return SkMemoryStream::MakeDirect(data, length).release();
}

extern "C" void C_SkDynamicMemoryWStream_Construct(SkDynamicMemoryWStream* uninitialized) {
    new (uninitialized) SkDynamicMemoryWStream();
}

extern "C" SkData* C_SkDynamicMemoryWStream_detachAsData(SkDynamicMemoryWStream* self) {
    return self->detachAsData().release();
}

extern "C" SkStreamAsset* C_SkDynamicMemoryWStream_detachAsStream(SkDynamicMemoryWStream* self) {
    return self->detachAsStream().release();
}

//
// core/SkTiledImageUtils.h
//

extern "C" void C_SkTiledImageUtils_DrawImageRect(
    SkCanvas* canvas,
    const SkImage* image,
    const SkRect& src,
    const SkRect& dst,
    const SkSamplingOptions* sampling,
    const SkPaint* paint,
    SkCanvas::SrcRectConstraint constraint) {
    SkTiledImageUtils::DrawImageRect(canvas, image, src, dst, *sampling, paint, constraint);
}

extern "C" void C_SkTiledImageUtils_GetImageKeyValues(
    const SkImage* image,
    uint32_t keyValues[SkTiledImageUtils::kNumImageKeyValues]
) {
    SkTiledImageUtils::GetImageKeyValues(image, keyValues);
}

//
// core/SkYUVAInfo.h
//

extern "C" void C_SkYUVAInfo_Construct(SkYUVAInfo* uninitialized) {
    new (uninitialized) SkYUVAInfo();
}

extern "C" void C_SkYUVAInfo_destruct(SkYUVAInfo* self) {
    self->~SkYUVAInfo();
}

extern "C" void C_SkYUVAInfo_SubsamplingFactors(SkYUVAInfo::Subsampling subsampling, int factors[2]) {
    auto f = SkYUVAInfo::SubsamplingFactors(subsampling);
    factors[0] = std::get<0>(f);
    factors[1] = std::get<1>(f);
}

extern "C" void C_SkYUVAInfo_PlaneSubsamplingFactors(SkYUVAInfo::PlaneConfig planeConfig, SkYUVAInfo::Subsampling subsampling, int planeIdx, int factors[2]) {
    auto f = SkYUVAInfo::PlaneSubsamplingFactors(planeConfig, subsampling, planeIdx);
    factors[0] = std::get<0>(f);
    factors[1] = std::get<1>(f);
}

extern "C" int C_SkYUVAInfo_NumPlanes(SkYUVAInfo::PlaneConfig planeConfig) {
    return SkYUVAInfo::NumPlanes(planeConfig);
}

extern "C" int C_SkYUVAInfo_NumChannelsInPlane(SkYUVAInfo::PlaneConfig planarConfig, int i) {
    return SkYUVAInfo::NumChannelsInPlane(planarConfig, i);
}

extern "C" bool C_SkYUVAInfo_equals(const SkYUVAInfo* a, const SkYUVAInfo* b) {
    return *a == *b;
}

extern "C" void C_SkYUVAInfo_makeSubsampling(const SkYUVAInfo* self, SkYUVAInfo::Subsampling subsampling, SkYUVAInfo* uninitialized) {
    new (uninitialized) SkYUVAInfo(self->makeSubsampling(subsampling));
}

extern "C" void C_SkYUVAInfo_makeDimensions(const SkYUVAInfo* self, const SkISize* dimensions, SkYUVAInfo* uninitialized) {
    new (uninitialized) SkYUVAInfo(self->makeDimensions(*dimensions));
}

//
// core/SkYUVAPixmaps.h
//

extern "C" void C_SkYUVAPixmapInfo_Construct(SkYUVAPixmapInfo* uninitialized) {
    new (uninitialized) SkYUVAPixmapInfo();
}

extern "C" void C_SkYUVAPixmapInfo_destruct(SkYUVAPixmapInfo* self) {
    self->~SkYUVAPixmapInfo();
}

extern "C" bool C_SkYUVAPixmapInfo_equals(const SkYUVAPixmapInfo* a, const SkYUVAPixmapInfo* b) {
    return *a == *b;
}

extern "C" size_t C_SkYUVAPixmapInfo_rowBytes(const SkYUVAPixmapInfo* self, int i) {
    return self->rowBytes(i);
}

extern "C" const SkImageInfo* C_SkYUVAPixmapInfo_planeInfo(const SkYUVAPixmapInfo* self, int i) {
    return &self->planeInfo(i);
}

extern "C" bool C_SkYUVAPixmapInfo_isValid(const SkYUVAPixmapInfo* self) {
    return self->isValid();
}

extern "C" void C_SkYUVAPixmapInfo_SupportedDataTypes_Construct(SkYUVAPixmapInfo::SupportedDataTypes* uninitialized) {
    new (uninitialized) SkYUVAPixmapInfo::SupportedDataTypes();
}

extern "C" void C_SkYUVAPixmapInfo_SupportedDataTypes_destruct(SkYUVAPixmapInfo::SupportedDataTypes* self) {
    self->~SupportedDataTypes();
}

extern "C" void C_SkYUVAPixmapInfo_SupportedDataTypes_All(SkYUVAPixmapInfo::SupportedDataTypes* uninitialized) {
    new (uninitialized) SkYUVAPixmapInfo::SupportedDataTypes(SkYUVAPixmapInfo::SupportedDataTypes::All());
}

extern "C" bool C_SkYUVAPixmapInfo_SupportedDataTypes_supported(
    const SkYUVAPixmapInfo::SupportedDataTypes* self, 
    SkYUVAPixmapInfo::PlaneConfig pc, 
    SkYUVAPixmapInfo::DataType dt) {
    return self->supported(pc, dt);
}

extern "C" SkColorType C_SkYUVAPixmapInfo_DefaultColorTypeForDataType(SkYUVAPixmapInfo::DataType dt, int numChannels) {
    return SkYUVAPixmapInfo::DefaultColorTypeForDataType(dt, numChannels);
}

extern "C" int C_SkYUVAPixmapInfo_NumChannelsAndDataType(SkColorType colorType, SkYUVAPixmapInfo::DataType* dataType) {
    auto numDT = SkYUVAPixmapInfo::NumChannelsAndDataType(colorType);
    *dataType = std::get<1>(numDT);
    return std::get<0>(numDT);
}

extern "C" void C_SkYUVAPixmaps_Construct(SkYUVAPixmaps* uninitialized) {
    new (uninitialized) SkYUVAPixmaps();
}

extern "C" void C_SkYUVAPixmaps_destruct(SkYUVAPixmaps* self) {
    self->~SkYUVAPixmaps();
}

extern "C" void C_SkYUVAPixmaps_MakeCopy(const SkYUVAPixmaps* self, SkYUVAPixmaps* uninitialized) {
    new (uninitialized) SkYUVAPixmaps(SkYUVAPixmaps::MakeCopy(*self));
}

extern "C" void C_SkYUVAPixmaps_Allocate(SkYUVAPixmaps* uninitialized, const SkYUVAPixmapInfo* yuvaPixmapInfo) {
    new (uninitialized) SkYUVAPixmaps(SkYUVAPixmaps::Allocate(*yuvaPixmapInfo));
}

extern "C" void C_SkYUVAPixmaps_FromData(SkYUVAPixmaps* uninitialized, const SkYUVAPixmapInfo* yuvaPixmapInfo, SkData* data) {
    new (uninitialized) SkYUVAPixmaps(SkYUVAPixmaps::FromData(*yuvaPixmapInfo, sp(data)));
}

extern "C" void C_SkYUVAPixmaps_FromExternalMemory(SkYUVAPixmaps* uninitialized, const SkYUVAPixmapInfo* yuvaPixmapInfo, void* memory) {
    new (uninitialized) SkYUVAPixmaps(SkYUVAPixmaps::FromExternalMemory(*yuvaPixmapInfo, memory));
}

extern "C" void C_SkYUVAPixmaps_FromExternalPixmaps(SkYUVAPixmaps* uninitialized, const SkYUVAInfo* yuvaInfo, const SkPixmap pixmaps[SkYUVAPixmaps::kMaxPlanes]) {
    new (uninitialized) SkYUVAPixmaps(SkYUVAPixmaps::FromExternalPixmaps(*yuvaInfo, pixmaps));
}

extern "C" void C_SkYUVAPixmaps_pixmapsInfo(const SkYUVAPixmaps* self, SkYUVAPixmapInfo* uninitialized) {
    new (uninitialized) SkYUVAPixmapInfo(self->pixmapsInfo());
}

extern "C" const SkPixmap* C_SkYUVAPixmaps_planes(const SkYUVAPixmaps* self) {
    return self->planes().data();
}

extern "C" bool C_SkYUVAPixmaps_isValid(const SkYUVAPixmaps* self) {
    return self->isValid();
}

//
// effects/
//

extern "C" void C_Effects_Types(SkTableMaskFilter *) {}

//
// effects/SkBlenders.h
//

extern "C" SkBlender* C_SkBlenders_Arithmetic(float k1, float k2, float k3, float k4, bool enforcePremul) {
    return SkBlenders::Arithmetic(k1, k2, k3, k4, enforcePremul).release();
}

//
// effects/SkGradient.h
//

extern "C" SkShader* C_SkShaders_LinearGradient(
    const SkPoint pts[2],
    const SkColor4f* colors,
    size_t colorCount,
    const float* pos,
    size_t posCount,
    SkTileMode mode,
    SkColorSpace* colorSpace,
    const SkGradient::Interpolation* interpolation,
    const SkMatrix* localMatrix
) {
    SkGradient::Colors gradColors(
        SkSpan<const SkColor4f>(colors, colorCount),
        SkSpan<const float>(pos, posCount),
        mode,
        sp(colorSpace)
    );
    
    SkGradient grad(gradColors, *interpolation);
    return SkShaders::LinearGradient(pts, grad, localMatrix).release();
}

extern "C" SkShader* C_SkShaders_RadialGradient(
    const SkPoint* center,
    float radius,
    const SkColor4f* colors,
    size_t colorCount,
    const float* pos,
    size_t posCount,
    SkTileMode mode,
    SkColorSpace* colorSpace,
    const SkGradient::Interpolation* interpolation,
    const SkMatrix* localMatrix
) {
    SkGradient::Colors gradColors(
        SkSpan<const SkColor4f>(colors, colorCount),
        SkSpan<const float>(pos, posCount),
        mode,
        sp(colorSpace)
    );
    
    SkGradient grad(gradColors, *interpolation);
    return SkShaders::RadialGradient(*center, radius, grad, localMatrix).release();
}

extern "C" SkShader* C_SkShaders_TwoPointConicalGradient(
    const SkPoint* start,
    float startRadius,
    const SkPoint* end,
    float endRadius,
    const SkColor4f* colors,
    size_t colorCount,
    const float* pos,
    size_t posCount,
    SkTileMode mode,
    SkColorSpace* colorSpace,
    const SkGradient::Interpolation* interpolation,
    const SkMatrix* localMatrix
) {
    SkGradient::Colors gradColors(
        SkSpan<const SkColor4f>(colors, colorCount),
        SkSpan<const float>(pos, posCount),
        mode,
        sp(colorSpace)
    );
    
    SkGradient grad(gradColors, *interpolation);
    return SkShaders::TwoPointConicalGradient(*start, startRadius, *end, endRadius, grad, localMatrix).release();
}

extern "C" SkShader* C_SkShaders_SweepGradient(
    const SkPoint* center,
    float startAngle,
    float endAngle,
    const SkColor4f* colors,
    size_t colorCount,
    const float* pos,
    size_t posCount,
    SkTileMode mode,
    SkColorSpace* colorSpace,
    const SkGradient::Interpolation* interpolation,
    const SkMatrix* localMatrix
) {
    SkGradient::Colors gradColors(
        SkSpan<const SkColor4f>(colors, colorCount),
        SkSpan<const float>(pos, posCount),
        mode,
        sp(colorSpace)
    );
    
    SkGradient grad(gradColors, *interpolation);
    return SkShaders::SweepGradient(*center, startAngle, endAngle, grad, localMatrix).release();
}

//
// effects/SkPerlinNoiseShader.h
//

extern "C" SkShader* C_SkShaders_MakeFractalNoise(SkScalar baseFrequencyX, SkScalar baseFrequencyY, int numOctaves, SkScalar seed, const SkISize* tileSize) {
    return SkShaders::MakeFractalNoise(baseFrequencyX, baseFrequencyY, numOctaves, seed, tileSize).release();
}

extern "C" SkShader* C_SkShaders_MakeTurbulence(SkScalar baseFrequencyX, SkScalar baseFrequencyY, int numOctaves, SkScalar seed, const SkISize* tileSize) {
    return SkShaders::MakeTurbulence(baseFrequencyX, baseFrequencyY, numOctaves, seed, tileSize).release();
}

//
// effects/SkPath1DPathEffect.h
//

extern "C" SkPathEffect* C_SkPath1DPathEffect_Make(const SkPath* path, SkScalar advance, SkScalar phase, SkPath1DPathEffect::Style style) {
    return SkPath1DPathEffect::Make(*path, advance, phase, style).release();
}

//
// effects/SkLine2DPathEffect.h
//

extern "C" SkPathEffect* C_SkLine2DPathEffect_Make(SkScalar width, const SkMatrix* matrix) {
    return SkLine2DPathEffect::Make(width, *matrix).release();
}

//
// effects/SkPath2DPathEffect.h
//

extern "C" SkPathEffect* C_SkPath2DPathEffect_Make(const SkMatrix* matrix, const SkPath* path) {
    return SkPath2DPathEffect::Make(*matrix, *path).release();
}

//
// effects/SkColorMatrix.h
//

extern "C" void C_SkColorMatrix_Construct(SkColorMatrix* uninitialized) {
    new (uninitialized)SkColorMatrix();
}

extern "C" void C_SkColorMatrix_Construct2(SkColorMatrix* uninitialized, 
                                           float m00, float m01, float m02, float m03, float m04,
                                           float m10, float m11, float m12, float m13, float m14,
                                           float m20, float m21, float m22, float m23, float m24,
                                           float m30, float m31, float m32, float m33, float m34) {
    new (uninitialized)SkColorMatrix(m00, m01, m02, m03, m04,
                                    m10, m11, m12, m13, m14,
                                    m20, m21, m22, m23, m24,
                                    m30, m31, m32, m33, m34);
}

extern "C" void C_SkColorMatrix_setRowMajor(SkColorMatrix* self, const float src[20]) {
    self->setRowMajor(src);
}

extern "C" void C_SkColorMatrix_getRowMajor(const SkColorMatrix* self, float dst[20]) {
    self->getRowMajor(dst);
}

//
// effects/SkCornerPathEffect.h
//

extern "C" SkPathEffect* C_SkCornerPathEffect_Make(SkScalar radius) {
    return SkCornerPathEffect::Make(radius).release();
}

//
// effects/SkDashPathEffect.h
//

extern "C" SkPathEffect* C_SkDashPathEffect_Make(const SkScalar intervals[], size_t count, SkScalar phase) {
    return SkDashPathEffect::Make(SkSpan(intervals, count), phase).release();
}

//
// effects/SkDiscretePathEffect.h
//

extern "C" SkPathEffect* C_SkDiscretePathEffect_Make(SkScalar segLength, SkScalar dev, uint32_t seedAssist) {
    return SkDiscretePathEffect::Make(segLength, dev, seedAssist).release();
}

//
// effects/SkHighContrastFilter.h
//

extern "C" SkColorFilter* C_SkHighContrastFilter_Make(const SkHighContrastConfig* config) {
    return SkHighContrastFilter::Make(*config).release();
}

//
// effects/SkLumaColorFilter.h
//

extern "C" SkColorFilter* C_SkLumaColorFilter_Make() {
    return SkLumaColorFilter::Make().release();
}

//
// effects/SkOverdrawColorFilter.h
//

extern "C" SkColorFilter* C_SkOverdrawColorFilter_MakeWithSkColors(const SkColor colors[SkOverdrawColorFilter::kNumColors]) {
    return SkOverdrawColorFilter::MakeWithSkColors(colors).release();
}

//
// effects/SkRuntimeEffect.h
//

extern "C" {

void C_SkRuntimeEffect_Options_Construct(SkRuntimeEffect::Options* uninitialized, bool forceUnoptimized, const char* name, size_t length) {
    new (uninitialized) SkRuntimeEffect::Options();
    uninitialized->forceUnoptimized = forceUnoptimized;
    uninitialized->fName = std::string_view(name, length);
}

SkRuntimeEffect *C_SkRuntimeEffect_MakeForColorFilter(
    const SkString *sksl,
    const SkRuntimeEffect::Options *options,
    SkString *error)
{
    auto r = SkRuntimeEffect::MakeForColorFilter(*sksl, *options);
    *error = r.errorText;
    return r.effect.release();
}

SkRuntimeEffect *C_SkRuntimeEffect_MakeForShader(
    const SkString *sksl,
    const SkRuntimeEffect::Options *options,
    SkString *error)
{
    auto r = SkRuntimeEffect::MakeForShader(*sksl, *options);
    *error = r.errorText;
    return r.effect.release();
}

SkRuntimeEffect *C_SkRuntimeEffect_MakeForBlender(
    const SkString *sksl,
    const SkRuntimeEffect::Options *options,
    SkString *error)
{
    auto r = SkRuntimeEffect::MakeForBlender(*sksl, *options);
    *error = r.errorText;
    return r.effect.release();
}

SkShader *C_SkRuntimeEffect_makeShader(
    const SkRuntimeEffect *self, const SkData *uniforms,
    SkRuntimeEffect::ChildPtr *children, size_t childCount,
    const SkMatrix *localMatrix)
{
    return self->makeShader(
        sp(uniforms),
        SkSpan<SkRuntimeEffect::ChildPtr>(children, childCount), 
        localMatrix).release();
}

SkColorFilter *C_SkRuntimeEffect_makeColorFilter(
    const SkRuntimeEffect *self, const SkData *inputs, SkRuntimeEffect::ChildPtr *children, size_t childCount)
{
    return self->makeColorFilter(sp(inputs), SkSpan<SkRuntimeEffect::ChildPtr>(children, childCount)).release();
}

SkBlender *C_SkRuntimeEffect_makeBlender(
    const SkRuntimeEffect *self, const SkData *uniforms, SkRuntimeEffect::ChildPtr *children, size_t childCount)
{
    return self->makeBlender(sp(uniforms), SkSpan<SkRuntimeEffect::ChildPtr>(children, childCount)).release();
}

const unsigned char* C_SkRuntimeEffect_source(const SkRuntimeEffect *self, size_t* len) {
    auto& str = self->source();
    *len = str.length();
    return reinterpret_cast<const unsigned char*>(str.data());
}

const SkRuntimeEffect::Uniform* C_SkRuntimeEffect_uniforms(const SkRuntimeEffect* self, size_t* count) {
    auto uniforms = self->uniforms();
    *count = uniforms.size();
    return uniforms.begin();
}

const SkRuntimeEffect::Child* C_SkRuntimeEffect_children(const SkRuntimeEffect* self, size_t* count) {
    auto children = self->children();
    *count = children.size();
    return children.begin();
}

const SkRuntimeEffect::Uniform* C_SkRuntimeEffect_findUniform(
    const SkRuntimeEffect* self, const char* name, size_t count) {
    return self->findUniform(std::string_view(name, count));
}

const SkRuntimeEffect::Child* C_SkRuntimeEffect_findChild(
    const SkRuntimeEffect* self, const char* name, size_t count) {
    return self->findChild(std::string_view(name, count));
}

bool C_SkRuntimeEffect_allowShader(const SkRuntimeEffect* self) {
    return self->allowShader();
}

bool C_SkRuntimeEffect_allowColorFilter(const SkRuntimeEffect* self) {
    return self->allowColorFilter();
}

bool C_SkRuntimeEffect_allowBlender(const SkRuntimeEffect* self) {
    return self->allowBlender();
}

void C_SkRuntimeShaderBuilder_Construct(SkRuntimeShaderBuilder *uninitialized,
                                        SkRuntimeEffect *effect) {
  new (uninitialized) SkRuntimeShaderBuilder(sp(effect));
}

void C_SkRuntimeShaderBuilder_destruct(SkRuntimeShaderBuilder *self) {
  self->~SkRuntimeShaderBuilder();
}

SkShader *
C_SkRuntimeShaderBuilder_makeShader(const SkRuntimeShaderBuilder *self,
                                    const SkMatrix *localMatrix) {
  auto shader = self->makeShader(localMatrix);
  return shader.release();
}

enum class ShaderBuilderUniformResult {
  Ok,
  Error
};

ShaderBuilderUniformResult C_SkRuntimeShaderBuilder_setUniformFloat(SkRuntimeShaderBuilder *self,
                                              const char *name, size_t count,
                                              const float *const values,
                                              size_t len) {
  using float2 = std::array<float, 2>;
  using float3 = std::array<float, 3>;
  using float4 = std::array<float, 4>;
  using float3x3 = std::array<float, 9>;
  using float4x4 = std::array<float, 16>;

  switch (len) {
  case 1:
    self->uniform(std::string_view(name, count)) = *values;
    return ShaderBuilderUniformResult::Ok;
  case 2:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const float2 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 3:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const float3 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 4:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const float4 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 9:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const float3x3 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 16:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const float4x4 *>(values);
    return ShaderBuilderUniformResult::Ok;
  }

  return ShaderBuilderUniformResult::Error;
}

ShaderBuilderUniformResult C_SkRuntimeShaderBuilder_setUniformInt(SkRuntimeShaderBuilder *self,
                                            const char *name, size_t count,
                                            const int *const values,
                                            size_t len) {
  using int2 = std::array<int, 2>;
  using int3 = std::array<int, 3>;
  using int4 = std::array<int, 4>;
  switch (len) {
  case 1:
    self->uniform(std::string_view(name, count)) = *values;
    return ShaderBuilderUniformResult::Ok;
  case 2:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const int2 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 3:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const int3 *>(values);
    return ShaderBuilderUniformResult::Ok;
  case 4:
    self->uniform(std::string_view(name, count)) =
        *reinterpret_cast<const int4 *>(values);
    return ShaderBuilderUniformResult::Ok;
  }
  return ShaderBuilderUniformResult::Error;
}
}

//
// effects/SkShaderMaskFilter.h
//

extern "C" SkMaskFilter* C_SkShaderMaskFilter_Make(SkShader* shader) {
    return SkShaderMaskFilter::Make(sp(shader)).release();
}

//
// effects/SkTrimPathEffect.h
//

extern "C" SkPathEffect *C_SkTrimPathEffect_Make(SkScalar startT, SkScalar stopT, SkTrimPathEffect::Mode mode) {
    return SkTrimPathEffect::Make(startT, stopT, mode).release();
}

//
// effects/SkImageFilters.h
//

extern "C" {

SkImageFilter *C_SkImageFilters_Arithmetic(float k1, float k2, float k3, float k4, bool enforcePMColor,
                                           SkImageFilter *background,
                                           SkImageFilter *foreground,
                                           const SkRect *cropRect) {
    return SkImageFilters::Arithmetic(k1, k2, k3, k4, enforcePMColor, sp(background),
                                      sp(foreground), cropRect).release();
}

SkImageFilter *C_SkImageFilters_Blend(SkBlender *blender,
                                      SkImageFilter *background,
                                      SkImageFilter *foreground,
                                      const SkRect *cropRect)
{
    return SkImageFilters::Blend(sp(blender), sp(background),
                                 sp(foreground), cropRect)
        .release();
}

SkImageFilter *C_SkImageFilters_Blur(SkScalar sigmaX, SkScalar sigmaY, SkTileMode tileMode,
                                     SkImageFilter *input, const SkRect *cropRect)
{
    return SkImageFilters::Blur(sigmaX, sigmaY, tileMode, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_ColorFilter(SkColorFilter *cf, SkImageFilter *input, const SkRect *cropRect) {
    return SkImageFilters::ColorFilter(sp(cf), sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_Compose(SkImageFilter *outer, SkImageFilter *inner) {
    return SkImageFilters::Compose(sp(outer), sp(inner)).release();
}

SkImageFilter *C_SkImageFilters_Crop(const SkRect* rect, SkTileMode tileMode, SkImageFilter *input) {
    return SkImageFilters::Crop(*rect, tileMode, sp(input)).release();
}

SkImageFilter *C_SkImageFilters_DisplacementMap(SkColorChannel xChannelSelector,
                                                SkColorChannel yChannelSelector,
                                                SkScalar scale, SkImageFilter *displacement,
                                                SkImageFilter *color,
                                                const SkRect *cropRect) {
    return SkImageFilters::DisplacementMap(xChannelSelector, yChannelSelector, scale, sp(displacement), sp(color),
                                           cropRect).release();
}

SkImageFilter *C_SkImageFilters_DropShadow(SkScalar dx, SkScalar dy,
                                           SkScalar sigmaX, SkScalar sigmaY,
                                           const SkColor4f* color, SkColorSpace *colorSpace, SkImageFilter *input,
                                           const SkRect *cropRect) {
    return SkImageFilters::DropShadow(dx, dy, sigmaX, sigmaY, *color, sp(colorSpace), sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_DropShadowOnly(SkScalar dx, SkScalar dy,
                                               SkScalar sigmaX, SkScalar sigmaY,
                                               const SkColor4f* color, SkColorSpace* colorSpace, SkImageFilter *input,
                                               const SkRect *cropRect) {
    return SkImageFilters::DropShadowOnly(dx, dy, sigmaX, sigmaY, *color, sp(colorSpace), sp(input), cropRect).release();
}

SkImageFilter* C_SkImageFilters_Empty() {
    return SkImageFilters::Empty().release();
}

SkImageFilter *C_SkImageFilters_Image(
    SkImage *image, const SkRect *srcRect,
    const SkRect *dstRect, const SkSamplingOptions *sampling)
{
    return SkImageFilters::Image(sp(image), *srcRect, *dstRect, *sampling).release();
}

SkImageFilter *C_SkImageFilters_Magnifier(
    const SkRect *lensBounds,
    SkScalar zoomAmount,
    SkScalar inset,
    const SkSamplingOptions *sampling,
    SkImageFilter *input,
    const SkRect *cropRect)
{
    return SkImageFilters::Magnifier(*lensBounds, zoomAmount, inset, *sampling, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_MatrixConvolution(const SkISize *kernelSize,
                                                  const SkScalar kernel[], SkScalar gain,
                                                  SkScalar bias, const SkIPoint *kernelOffset,
                                                  SkTileMode tileMode, bool convolveAlpha,
                                                  SkImageFilter *input,
                                                  const SkRect *cropRect) {
    return SkImageFilters::MatrixConvolution(*kernelSize, kernel, gain, bias, *kernelOffset, tileMode, convolveAlpha,
                                             sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_MatrixTransform(const SkMatrix *matrix,
                                                const SkSamplingOptions *sampling,
                                                SkImageFilter *input)
{
    return SkImageFilters::MatrixTransform(*matrix, *sampling, sp(input)).release();
}

SkImageFilter *C_SkImageFilters_Merge(SkImageFilter *const filters[], int count,
                                      const SkRect *cropRect) {
    auto array = new sk_sp<SkImageFilter>[count];
    for (int i = 0; i < count; ++i) {
        array[i] = sp(filters[i]);
    }
    auto imageFilter = SkImageFilters::Merge(array, count, cropRect).release();
    delete[] array;
    return imageFilter;
}

SkImageFilter *C_SkImageFilters_Offset(SkScalar dx, SkScalar dy, SkImageFilter *input,
                                       const SkRect *cropRect) {
    return SkImageFilters::Offset(dx, dy, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_Picture(SkPicture *pic, const SkRect *targetRect) {
    return SkImageFilters::Picture(sp(pic), *targetRect).release();
}

SkImageFilter *C_SkImageFilters_Shader(SkShader *shader,
                                       SkImageFilters::Dither dither,
                                       const SkRect *cropRect)
{
    return SkImageFilters::Shader(sp(shader), dither, cropRect).release();
}

SkImageFilter *C_SkImageFilters_Tile(const SkRect *src, const SkRect *dst,
                                     SkImageFilter *input) {
    return SkImageFilters::Tile(*src, *dst, sp(input)).release();
}

SkImageFilter *C_SkImageFilters_Dilate(SkScalar radiusX, SkScalar radiusY, SkImageFilter *input,
                                    const SkRect *cropRect) {
    return SkImageFilters::Dilate(radiusX, radiusY, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_Erode(SkScalar radiusX, SkScalar radiusY, SkImageFilter *input,
                                      const SkRect *cropRect) {
    return SkImageFilters::Erode(radiusX, radiusY, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_DistantLitDiffuse(const SkPoint3 *direction, SkColor lightColor,
                                                  SkScalar surfaceScale, SkScalar kd,
                                                  SkImageFilter *input,
                                                  const SkRect *cropRect)
{
    return SkImageFilters::DistantLitDiffuse(*direction, lightColor, surfaceScale, kd, sp(input), cropRect).release();
}

SkImageFilter *C_SkImageFilters_PointLitDiffuse(const SkPoint3 *direction, SkColor lightColor,
                                                SkScalar surfaceScale, SkScalar kd,
                                                SkImageFilter *input,
                                                const SkRect *cropRect) {
    return SkImageFilters::PointLitDiffuse(*direction, lightColor, surfaceScale, kd, sp(input), cropRect).release();
}

SkImageFilter *
C_SkImageFilters_SpotLitDiffuse(const SkPoint3 *location,
                                const SkPoint3 *target, SkScalar specularExponent, SkScalar cutoffAngle,
                                SkColor lightColor, SkScalar surfaceScale, SkScalar kd,
                                SkImageFilter *input, const SkRect *cropRect) {
    return SkImageFilters::SpotLitDiffuse(*location, *target, specularExponent, cutoffAngle, lightColor,
                                          surfaceScale, kd, sp(input), cropRect).release();
}

SkImageFilter *
C_ImageFilters_DistantLitSpecular(const SkPoint3 *direction,
                                  SkColor lightColor, SkScalar surfaceScale, SkScalar ks,
                                  SkScalar shininess, SkImageFilter *input,
                                  const SkRect *cropRect) {
    return SkImageFilters::DistantLitSpecular(*direction, lightColor, surfaceScale, ks, shininess,
                                              sp(input), cropRect).release();
}

SkImageFilter *
C_SkImageFilters_PointLitSpecular(const SkPoint3 &location,
                                  SkColor lightColor, SkScalar surfaceScale, SkScalar ks,
                                  SkScalar shininess, SkImageFilter *input,
                                  const SkRect *cropRect) {
    return SkImageFilters::PointLitSpecular(location, lightColor, surfaceScale, ks, shininess,
                                            sp(input), cropRect).release();
}

SkImageFilter *
C_SkImageFilters_SpotLitSpecular(const SkPoint3 &location,
                                 const SkPoint3 &target, SkScalar specularExponent, SkScalar cutoffAngle,
                                 SkColor lightColor, SkScalar surfaceScale, SkScalar ks,
                                 SkScalar shininess, SkImageFilter *input,
                                 const SkRect *cropRect) {
    return SkImageFilters::SpotLitSpecular(location, target, specularExponent, cutoffAngle, lightColor,
                                           surfaceScale, ks, shininess, sp(input),
                                           cropRect).release();
}

SkImageFilter *C_SkImageFilters_RuntimeShader(
    const SkRuntimeShaderBuilder &builder, const char *childShaderName,
    size_t childShaderNameCount, SkImageFilter *input) {
  auto imageFilter = SkImageFilters::RuntimeShader(
      builder, std::string_view(childShaderName, childShaderNameCount),
      sp(input));
  return imageFilter.release();
}
}

//
// encode/
//

extern "C" {

// SkPngEncoder

bool C_SkPngEncoder_Encode(SkWStream* stream, const SkPixmap* pixmap,
    SkDataTable* comments, SkPngEncoder::FilterFlag filterFlags, int zLibLevel) {

    SkPngEncoder::Options options;
    options.fComments = sp(comments);
    options.fFilterFlags = filterFlags;
    options.fZLibLevel = zLibLevel;

    return SkPngEncoder::Encode(stream, *pixmap, options);
}

SkData* C_SkPngEncoder_EncodePixmap(
    const SkPixmap* src,
    SkDataTable* comments, 
    SkPngEncoder::FilterFlag filterFlags, 
    int zLibLevel) {

    SkPngEncoder::Options options;
    options.fComments = sp(comments);
    options.fFilterFlags = filterFlags;
    options.fZLibLevel = zLibLevel;

    return SkPngEncoder::Encode(*src, options).release();
}

SkData* C_SkPngEncoder_EncodeImage(GrDirectContext* ctx, const SkImage* img,
    SkDataTable* comments, SkPngEncoder::FilterFlag filterFlags, int zLibLevel) {

    SkPngEncoder::Options options;
    options.fComments = sp(comments);
    options.fFilterFlags = filterFlags;
    options.fZLibLevel = zLibLevel;

    return SkPngEncoder::Encode(ctx, img, options).release();
}

// SkPngRustEncoder

/*
bool C_SkPngRustEncoder_Encode(
    SkWStream* stream,
    const SkPixmap* pixmap,
    SkPngRustEncoder::CompressionLevel compressionLevel,
    SkDataTable* comments) {

    SkPngRustEncoder::Options options;
    options.fCompressionLevel = compressionLevel;
    options.fComments = sp(comments);

    return SkPngRustEncoder::Encode(stream, *pixmap, options);
}
*/

// SkJpegEncoder

#if defined(SK_CODEC_ENCODES_JPEG)
bool C_SkJpegEncoder_Encode(SkWStream* stream, const SkPixmap* pixmap, 
    int quality,
    SkJpegEncoder::Downsample downsample, 
    SkJpegEncoder::AlphaOption alphaOption, 
    const SkData* xmpMetadata, 
    const SkEncodedOrigin* origin) {
    SkJpegEncoder::Options options;
    options.fQuality = quality;
    options.fDownsample = downsample;
    options.fAlphaOption = alphaOption;
    options.xmpMetadata = xmpMetadata;
    if (origin) {
        options.fOrigin = *origin;
    }

    return SkJpegEncoder::Encode(stream, *pixmap, options);
}

bool C_SkJpegEncoder_EncodeYUVAPixmaps(SkWStream* stream, const SkYUVAPixmaps* src,
    const SkColorSpace* srcColorSpace,
    int quality,
    SkJpegEncoder::Downsample downsample, 
    SkJpegEncoder::AlphaOption alphaOption, 
    const SkData* xmpMetadata,
    const SkEncodedOrigin* origin) {
    SkJpegEncoder::Options options;
    options.fQuality = quality;
    options.fDownsample = downsample;
    options.fAlphaOption = alphaOption;
    options.xmpMetadata = xmpMetadata;
    if (origin) {
        options.fOrigin = *origin;
    }

    return SkJpegEncoder::Encode(stream, *src, srcColorSpace, options);
}

SkData* C_SkJpegEncoder_EncodePixmap(const SkPixmap* src,
    int quality,
    SkJpegEncoder::Downsample downsample, 
    SkJpegEncoder::AlphaOption alphaOption, 
    const SkData* xmpMetadata,
    const SkEncodedOrigin* origin) {
    SkJpegEncoder::Options options;
    options.fQuality = quality;
    options.fDownsample = downsample;
    options.fAlphaOption = alphaOption;
    options.xmpMetadata = xmpMetadata;
    if (origin) {
        options.fOrigin = *origin;
    }

    return SkJpegEncoder::Encode(*src, options).release();
}

SkData* C_SkJpegEncoder_EncodeImage(GrDirectContext* ctx, const SkImage* img, 
    int quality,
    SkJpegEncoder::Downsample downsample, 
    SkJpegEncoder::AlphaOption alphaOption, 
    const SkData* xmpMetadata,
    const SkEncodedOrigin* origin) {
    SkJpegEncoder::Options options;
    options.fQuality = quality;
    options.fDownsample = downsample;
    options.fAlphaOption = alphaOption;
    options.xmpMetadata = xmpMetadata;
    if (origin) {
        options.fOrigin = *origin;
    }

    return SkJpegEncoder::Encode(ctx, img, options).release();
}
#endif

} // extern "C"

//
// docs/SkPDFDocument.h
//

#if defined(SK_SUPPORT_PDF)
extern "C" void C_SkPDF_AttributeList_destruct(SkPDF::AttributeList *self) {
    self->~AttributeList();
}

extern "C" void C_SkPDF_AttributeList_appendFloatArray(SkPDF::AttributeList *self, const char *owner, const char *name, const float *const value, size_t len) {
    std::vector<float> v(value, value + len);
    self->appendFloatArray(owner, name, v);
}

extern "C" void C_SkPDF_AttributeList_appendName(SkPDF::AttributeList *self, const char *owner, const char *name, const char *value) {
    self->appendName(owner, name, value);
}

extern "C" void C_SkPDF_AttributeList_appendTextString(SkPDF::AttributeList *self, const char *owner, const char *name, const char *value) {
    self->appendTextString(owner, name, value);
}

extern "C" void C_SkPDF_AttributeList_appendNodeIdArray(SkPDF::AttributeList *self, const char *owner, const char *name, const int *nodeIds, size_t len) {
    std::vector<int> ids(nodeIds, nodeIds + len);
    self->appendNodeIdArray(owner, name, ids);
}

extern "C" SkPDF::StructureElementNode *C_SkPDF_StructureElementNode_new() {
    return new SkPDF::StructureElementNode();
}

extern "C" void C_SkPDF_StructureElementNode_delete(SkPDF::StructureElementNode *self) {
    delete self;
}

extern "C" void C_SkPDF_StructureElementNode_setChildVector(SkPDF::StructureElementNode *self, SkPDF::StructureElementNode **nodes, size_t len)
{
    self->fChildVector = std::vector<std::unique_ptr<SkPDF::StructureElementNode>>();
    self->fChildVector.reserve(len);
    for (size_t i = 0; i != len; ++i)
    {
        auto node = nodes[i];
        nodes[i] = nullptr;
        self->fChildVector.push_back(std::unique_ptr<SkPDF::StructureElementNode>(node));
    }
}

extern "C" void C_SkPDF_StructElementNode_appendChild(SkPDF::StructureElementNode *self, SkPDF::StructureElementNode *node)
{
    self->fChildVector.push_back(std::unique_ptr<SkPDF::StructureElementNode>(node));
}

extern "C" size_t C_SkPDF_StructureElementNode_getChildVector(const SkPDF::StructureElementNode *self, const SkPDF::StructureElementNode ** nodes)
{
    if (self->fChildVector.empty())
    {
        *nodes = nullptr;
        return 0;
    }
    else
    {
        using ElementType = decltype(self->fChildVector)::value_type;
        static_assert(sizeof(ElementType) == sizeof(SkPDF::StructureElementNode*), "Size mismatch");
        static_assert(alignof(ElementType) == alignof(SkPDF::StructureElementNode*), "Alignment mismatch");

        *nodes = reinterpret_cast<SkPDF::StructureElementNode const *>(self->fChildVector.data());
        return self->fChildVector.size();
    }
}

extern "C" void C_SkPDF_Metadata_Construct(SkPDF::Metadata* uninitialized) {
    new (uninitialized)SkPDF::Metadata();
}

extern "C" void C_SkPDF_Metadata_destruct(SkPDF::Metadata* self) {
    self->~Metadata();
}

extern "C" SkDocument* C_SkPDF_MakeDocument(SkWStream* stream, const SkPDF::Metadata* metadata) {
    // We want to support JPeg encoding / decoding by default.
    SkPDF::Metadata meta = *metadata;
    meta.jpegDecoder = SkPDF::JPEG::Decode;
    meta.jpegEncoder = SkPDF::JPEG::Encode;
    return SkPDF::MakeDocument(stream, meta).release();
}

extern "C" void C_SkPDF_SetNodeId(SkCanvas* dst, int nodeID) {
    return SkPDF::SetNodeId(dst, nodeID);
}
#endif

//
// pathops/
//

extern "C" void C_SkOpBuilder_Construct(SkOpBuilder* uninitialized) {
    new (uninitialized) SkOpBuilder();
}

extern "C" void C_SkOpBuilder_destruct(SkOpBuilder* self) {
    self->~SkOpBuilder();
}

extern "C" bool C_SkOpBuilder_resolve(SkOpBuilder* self, SkPath* uninitialized) {
    if (auto res = self->resolve()) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" bool C_SkPathOps_Op(const SkPath* one, const SkPath* two, SkPathOp op, SkPath* uninitialized) {
    if (auto res = Op(*one, *two, op)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" bool C_SkPathOps_Simplify(const SkPath* path, SkPath* uninitialized) {
    if (auto res = Simplify(*path)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

extern "C" bool C_SkPathOps_AsWinding(const SkPath* path, SkPath* uninitialized) {
    if (auto res = AsWinding(*path)) { new (uninitialized) SkPath(); uninitialized->swap(*res); return true; }
    return false;
}

//
// svg/
//

extern "C" void C_SVG_Types(SkSVGCanvas *) {}

extern "C" SkCanvas* C_SkSVGCanvas_Make(const SkRect* bounds, SkWStream* writer, uint32_t flags) {
    return SkSVGCanvas::Make(*bounds, writer, flags).release();
}

//
// utils
//

extern "C" void C_Utils_Types(
        SkShadowFlags *,
        SkShadowUtils *,
        SkTextUtils *,
        SkParsePath *,
        SkCustomTypefaceBuilder *) {}

extern "C" Sk3DView* C_Sk3DView_new() {
    return new Sk3DView();
}

extern "C" void C_Sk3DView_delete(Sk3DView* self) {
    delete self;
}

extern "C" void C_SkCustomTypefaceBuilder_destruct(SkCustomTypefaceBuilder *self) {
    self->~SkCustomTypefaceBuilder();
}

extern "C" SkTypeface *C_SkCustomTypefaceBuilder_detach(SkCustomTypefaceBuilder *self) {
    return self->detach().release();
}

extern "C" void
C_SkCustomTypefaceBuilder_setGlyph(SkCustomTypefaceBuilder *self, SkGlyphID glyph, float advance, SkDrawable* drawable, const SkRect* bounds) {
    self->setGlyph(glyph, advance, sp(drawable), *bounds);
}

extern "C" SkTypeface* C_SkCustomTypefaceBuilder_FromData(SkData* data, const SkFontArguments* fontArguments) {
    return SkCustomTypefaceBuilder::MakeFromStream(SkMemoryStream::Make(sp(data)), *fontArguments).release();
}

extern "C" SkCanvas* C_SkMakeNullCanvas() {
    return SkMakeNullCanvas().release();
}

extern "C" SkOrderedFontMgr* C_SkOrderedFontMgr_new() {
    return new SkOrderedFontMgr();
}

extern "C" void C_SkOrderedFontMgr_append(SkOrderedFontMgr* self, SkFontMgr* fontMgr) {
    self->append(sp(fontMgr));
}

extern "C" void C_SkParsePath_ToSVGString(const SkPath* self, SkString* uninitialized, SkParsePath::PathEncoding encoding) {
    new (uninitialized) SkString(SkParsePath::ToSVGString(*self, encoding));
}

extern "C" bool C_SkParsePath_FromSVGString(const char* str, SkPath* out) {
    if (auto res = SkParsePath::FromSVGString(str)) { new (out) SkPath(); out->swap(*res); return true; }
    return false;
}

//
// SkStream <-> RustStream interop
//

class RustStream : public SkStream {
    void *m_data;
    size_t m_length;
    bool m_isEof;

    size_t (*m_read)(void *, void *, size_t);

    bool (*m_seekAbsolute)(void *, size_t);

    bool (*m_seekRelative)(void *, long);

public:
    RustStream(
        void *data,
        size_t length,
        size_t (*read)(void *, void *, size_t),
        bool (*seekAbsolute)(void *, size_t),
        bool (*seekRelative)(void *, long)
    );

    size_t read(void *buffer, size_t count);

    bool rewind();

    bool seek(size_t pos);

    bool move(long offset);

    bool isAtEnd() const;

    bool hasLength() const;

    size_t getLength() const;
};

RustStream::RustStream(
    void *data,
    size_t length,
    size_t (*read)(void *, void *, size_t),
    bool (*seekAbsolute)(void *, size_t),
    bool (*seekRelative)(void *, long)
) :
    m_data(data),
    m_length(length),
    m_isEof(false),
    m_read(read),
    m_seekAbsolute(seekAbsolute),
    m_seekRelative(seekRelative) 
{}

size_t RustStream::read(void *buffer, size_t count) {
    if (this->m_isEof) return 0;

    size_t out = (this->m_read)(this->m_data, buffer, count);

    if (!out) {
        this->m_isEof = true;
    }

    return out;
}

bool RustStream::rewind() {
    return this->seek(0);
}

bool RustStream::seek(size_t pos) {
    if (this->m_seekAbsolute) {
        return (this->m_seekAbsolute)(this->m_data, pos);
    } else {
        return false;
    }
}

bool RustStream::move(long offset) {
    if (this->m_seekRelative) {
        return (this->m_seekRelative)(this->m_data, offset);
    } else {
        return false;
    }
}

bool RustStream::hasLength() const {
    return this->m_length != (size_t) - 1;
}

size_t RustStream::getLength() const {
    return this->m_length;
}

bool RustStream::isAtEnd() const {
    return this->m_isEof;
}

extern "C" RustStream* C_RustStream_new(
    void *data,
    size_t length,
    size_t (*read)(void *, void *, size_t),
    bool (*seekAbsolute)(void *, size_t),
    bool (*seekRelative)(void *, long)
) {
    return new RustStream(data, length, read, seekAbsolute, seekRelative);
}

extern "C" void C_RustStream_delete(
    RustStream *stream
) {

    delete stream;
}

class RustWStream : public SkWStream {
    void *m_data;
    size_t m_length;

    bool (*m_write)(void *, const void *, size_t);
    void (*m_flush)(void *);

public:
    RustWStream(
        void* data,
        bool (*write)(void *, const void *, size_t),
        void (*flush)(void *)
    ) :
        m_data(data),
        m_write(write),
        m_flush(flush) 
    {}

    bool write(const void *buffer, size_t count) {
        auto r = (this->m_write)(this->m_data, buffer, count);
        if (r) {
            this->m_length += count;
        }
        return r;
    }

    void flush() {
        (this->m_flush)(this->m_data);
    }

    size_t bytesWritten() const {
        return this->m_length;
    }
};

extern "C" void C_RustWStream_construct(
    RustWStream *uninitialized,
    void *data,
    bool (*write)(void *, const void *, size_t),
    void (*flush)(void *)
) {
    new (uninitialized) RustWStream(data, write, flush);
}

extern "C" void C_RustWStream_destruct(RustWStream *stream) {
    stream->~RustWStream();
}
