#include "bindings.h"

#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkCPURecorder.h"
#include "include/core/SkDrawable.h"
#include "include/core/SkPicture.h"
#include "include/core/SkSurface.h"
#include "include/core/SkImageGenerator.h"
#include "include/gpu/MutableTextureState.h"
#include "include/gpu/ganesh/GrBackendSemaphore.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/GrYUVABackendTextures.h"
#include "include/gpu/ganesh/SkImageGanesh.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"

// Size / Alignment proxy for GrContextOptions
typedef std::optional<uint64_t> OptionalU64;

extern "C" void C_GpuUnreferencedTypes(skgpu::Origin *, OptionalU64*) {}

//
// core/SkSurface.h
//

extern "C" bool C_SkSurface_replaceBackendTexture(
        SkSurface* self,
        const GrBackendTexture* backendTexture,
        GrSurfaceOrigin origin,
        SkSurface::ContentChangeMode contentChangeMode) {
    return self->replaceBackendTexture(*backendTexture, origin, contentChangeMode);
}

//
// gpu/GrBackendSurface.h
//

// GrBackendSemaphore

extern "C" void C_GrBackendSemaphore_Construct(GrBackendSemaphore* uninitialized) {
    new(uninitialized) GrBackendSemaphore();
}

extern "C" void C_GrBackendSemaphore_CopyConstruct(
    GrBackendSemaphore* uninitialized,
    const GrBackendSemaphore* semaphore) {
    new(uninitialized) GrBackendSemaphore(*semaphore);
}

extern "C" void C_GrBackendSemaphore_destruct(GrBackendSemaphore* semaphore) {
    semaphore->~GrBackendSemaphore();
}

extern "C" GrBackendApi C_GrBackendSemaphore_backend(const GrBackendSemaphore* semaphore) {
    return semaphore->backend();
}

extern "C" bool C_GrBackendSemaphore_isInitialized(const GrBackendSemaphore* semaphore) {
    return semaphore->isInitialized();
}

// GrBackendRenderTarget

extern "C" void C_GrBackendRenderTarget_Construct(GrBackendRenderTarget* uninitialized) {
    new(uninitialized) GrBackendRenderTarget();
}

extern "C" void C_GrBackendRenderTarget_CopyConstruct(GrBackendRenderTarget* uninitialized, const GrBackendRenderTarget* renderTarget) {
    new(uninitialized) GrBackendRenderTarget(*renderTarget);
}

extern "C" void C_GrBackendRenderTarget_destruct(GrBackendRenderTarget* self) {
    self->~GrBackendRenderTarget();
}

extern "C" void C_GrBackendRenderTarget_getBackendFormat(const GrBackendRenderTarget* self, GrBackendFormat* uninitialized) {
    new(uninitialized) GrBackendFormat(self->getBackendFormat());
}

// GrBackendTexture

extern "C" GrBackendTexture* C_GrBackendTexture_new() {
    return new GrBackendTexture();
}

extern "C" GrBackendTexture* C_GrBackendTexture_Clone(const GrBackendTexture* texture) {
    return new GrBackendTexture(*texture);
}

extern "C" void C_GrBackendTexture_delete(const GrBackendTexture* self) {
    delete self;
}

extern "C" void C_GrBackendTexture_getBackendFormat(const GrBackendTexture* self, GrBackendFormat* format) {
    *format = self->getBackendFormat();
}

// GrBackendFormat

extern "C" void C_GrBackendFormat_Construct(GrBackendFormat* uninitialized) {
    new(uninitialized)GrBackendFormat();
}

extern "C" void C_GrBackendFormat_destruct(GrBackendFormat* self) {
    self->~GrBackendFormat();
}

extern "C" bool C_GrBackendFormat_Equals(const GrBackendFormat* lhs, const GrBackendFormat* rhs) {
    return *lhs == *rhs;
}

extern "C" void C_GrBackendFormat_makeTexture2D(const GrBackendFormat* self, GrBackendFormat* format) {
    *format = self->makeTexture2D();
}

//
// gpu/MutableTextureState.h
//

extern "C" skgpu::MutableTextureState* C_MutableTextureState_Construct() {
    return new skgpu::MutableTextureState();
}

extern "C" skgpu::MutableTextureState* C_MutableTextureState_CopyConstruct(const skgpu::MutableTextureState* state) {
    return new skgpu::MutableTextureState(*state);
}

extern "C" skgpu::BackendApi C_MutableTextureState_backend(const skgpu::MutableTextureState* self) {
    return self->backend();
}

//
// gpu/GrRecordingContext.h
//

// GrContext_Base.h
extern "C" GrDirectContext* C_GrRecordingContext_asDirectContext(GrRecordingContext* self) {
    return self->asDirectContext();
}

// GrContext_Base.h
extern "C" GrBackendApi C_GrRecordingContext_backend(const GrRecordingContext* self) {
    return self->backend();
}

extern "C" void C_GrRecordingContext_defaultBackendFormat(const GrRecordingContext* self, SkColorType ct, GrRenderable renderable, GrBackendFormat* result) {
    *result = self->defaultBackendFormat(ct, renderable);
}

// GrContext_Base.h
extern "C" void C_GrRecordingContext_compressedBackendFormat(const GrRecordingContext* self, SkTextureCompressionType compressionType, GrBackendFormat* backendFormat) {
    *backendFormat = self->compressedBackendFormat(compressionType);
}

extern "C" bool C_GrRecordingContext_abandoned(GrRecordingContext* self) {
    return self->abandoned();
}

extern "C" int C_GrRecordingContext_maxSurfaceSampleCountForColorType(const GrRecordingContext* self, SkColorType colorType) {
    return self->maxSurfaceSampleCountForColorType(colorType);
}

extern "C" skcpu::Recorder* C_GrRecordingContext_makeCPURecorder(GrRecordingContext* self) {
    return self->makeCPURecorder().release();
}

//
// gpu/GrDirectContext.h
//

extern "C" void C_GrDirectContext_flushAndSubmit(GrDirectContext* self) {
    self->flushAndSubmit();
}

extern "C" GrSemaphoresSubmitted C_GrDirectContext_flushImageWithInfo(GrDirectContext* self, SkImage* image, const GrFlushInfo* info) {
    return self->flush(sp(image), *info);
}

extern "C" void C_GrDirectContext_flushImage(GrDirectContext* self, SkImage* image) {
    self->flush(sp(image));
}

extern "C" void C_GrDirectContext_flushAndSubmitImage(GrDirectContext* self, SkImage* image) {
    self->flushAndSubmit(sp(image));
}

extern "C" void C_GrDirectContext_compressedBackendFormat(const GrDirectContext* self, SkTextureCompressionType compression, GrBackendFormat* result) {
    *result = self->compressedBackendFormat(compression);
}

extern "C" void C_GrDirectContext_directContextId(const GrDirectContext* self, GrDirectContext::DirectContextID* r) {
    *r = self->directContextID();
}

extern "C" void C_GrDirectContext_performDeferredCleanup(GrDirectContext* self, long msNotUsed, GrPurgeResourceOptions opts) {
    self->performDeferredCleanup(std::chrono::milliseconds(msNotUsed), opts);
}

//
// gpu/GrContextOptions.h
//

extern "C" void C_GrContextOptions_Construct(GrContextOptions* uninitialized) {
    new(uninitialized) GrContextOptions();
}

//
// gpu/GrRecordingContext.h
//

extern "C" bool C_GrRecordingContext_colorTypeSupportedAsSurface(const GrRecordingContext* self, SkColorType colorType) {
    return self->colorTypeSupportedAsSurface(colorType);
}

//
// gpu/GrYUVABackendTextures.h
//

extern "C" void C_GrYUVABackendTextureInfo_destruct(GrYUVABackendTextureInfo* self) {
    self->~GrYUVABackendTextureInfo();
}

extern "C" void C_GrYUVABackendTextureInfo_CopyConstruct(GrYUVABackendTextureInfo* uninitialized, const GrYUVABackendTextureInfo* src) {
    new(uninitialized) GrYUVABackendTextureInfo(*src);
}

extern "C" bool C_GrYUVABackendTextureInfo_equals(const GrYUVABackendTextureInfo* a, const GrYUVABackendTextureInfo* b) {
    return *a == *b;
}

extern "C" GrYUVABackendTextures* C_GrYUVABackendTextures_new(
    const SkYUVAInfo& yuvaInfo,
    const GrBackendTexture* const *backend_textures,
    GrSurfaceOrigin textureOrigin
) {
    GrBackendTexture textures[SkYUVAInfo::kMaxPlanes];
    for (int i = 0; i < SkYUVAInfo::kMaxPlanes; ++i) {
        textures[i] = *backend_textures[i];
    }
    return new GrYUVABackendTextures(yuvaInfo, textures, textureOrigin);
}

extern "C" void C_GrYUVABackendTextures_delete(GrYUVABackendTextures* self) {
    delete self;
}

extern "C" const SkYUVAInfo* C_GrYUVABackendTextures_yuvaInfo(const GrYUVABackendTextures* self) {
    return &self->yuvaInfo();
}

extern "C" GrSurfaceOrigin C_GrYUVABackendTextures_textureOrigin(const GrYUVABackendTextures* self) {
    return self->textureOrigin();
}

extern "C" const GrBackendTexture* C_GrYUVABackendTextures_textures(const GrYUVABackendTextures* self) {
    return self->textures().data();
}

//
// core/SkCanvas.h
//

extern "C" GrRecordingContext* C_SkCanvas_recordingContext(const SkCanvas* self) {
    return self->recordingContext();
}

//
// core/SkDrawable.h
//

extern "C" SkDrawable::GpuDrawHandler *C_SkDrawable_snapGpuDrawHandler(SkDrawable *self, GrBackendApi backendApi,
                                                                       const SkMatrix *matrix,
                                                                       const SkIRect *clipBounds,
                                                                       const SkImageInfo *bufferInfo) {
    return self->snapGpuDrawHandler(backendApi, *matrix, *clipBounds, *bufferInfo).release();
}

extern "C" void C_SkDrawable_GpuDrawHandler_delete(SkDrawable::GpuDrawHandler *self) {
    delete self;
}

extern "C" void C_SkDrawable_GpuDrawHandler_draw(SkDrawable::GpuDrawHandler *self, const GrBackendDrawableInfo *info) {
    self->draw(*info);
}

//
// gpu/ganesh/SkImageGanesh.h
//

extern "C" SkImage* C_SkImages_AdoptTextureFrom(
        GrRecordingContext* context,
        const GrBackendTexture* backendTexture,
        GrSurfaceOrigin origin,
        SkColorType colorType,
        SkAlphaType alphaType,
        SkColorSpace* colorSpace) {
    return SkImages::AdoptTextureFrom(context, *backendTexture, origin, colorType, alphaType, sp(colorSpace)).release();
}

extern "C" SkImage* C_SkImages_BorrowTextureFrom(
        GrRecordingContext* context,
        const GrBackendTexture* backendTexture,
        GrSurfaceOrigin origin,
        SkColorType colorType,
        SkAlphaType alphaType,
        SkColorSpace* colorSpace) {
    return SkImages::BorrowTextureFrom(context, *backendTexture, origin, colorType, alphaType, sp(colorSpace)).release();
}

extern "C" SkImage* C_SkImages_CrossContextTextureFromPixmap(
        GrDirectContext* context,
        const SkPixmap* pixmap,
        bool buildMips,
        bool limitToMaxTextureSize) {
    return SkImages::CrossContextTextureFromPixmap(context, *pixmap, buildMips, limitToMaxTextureSize).release();
}

extern "C" SkImage *C_SkImages_TextureFromCompressedTextureData(GrDirectContext *context, SkData *data, int width, int height,
                                                SkTextureCompressionType type, skgpu::Mipmapped mipMapped,
                                                GrProtected prot) {
    return SkImages::TextureFromCompressedTextureData(context, sp(data), width, height, type, mipMapped, prot).release();
}

extern "C" SkImage* C_SkImages_TextureFromImage(
        GrDirectContext* context,
        const SkImage* self,
        skgpu::Mipmapped mipMapped,
        skgpu::Budgeted budgeted) {
    return SkImages::TextureFromImage(context, self, mipMapped, budgeted).release();
}

extern "C" SkImage* C_SkImages_TextureFromYUVAPixmaps(
    GrRecordingContext* context,
    const SkYUVAPixmaps* pixmaps,
    skgpu::Mipmapped buildMips,
    bool limitToMaxTextureSize,
    SkColorSpace* imageColorSpace
) {
    return SkImages::TextureFromYUVAPixmaps(context, *pixmaps, buildMips, limitToMaxTextureSize, sp(imageColorSpace)).release();
}


extern "C" SkImage* C_SkImages_TextureFromYUVATextures(
    GrRecordingContext* context,
    const GrYUVABackendTextures* yuvaTextures,
    SkColorSpace* imageColorSpace) {
    return SkImages::TextureFromYUVATextures(context, *yuvaTextures, sp(imageColorSpace)).release();
}

extern "C" GrBackendTexture* C_SkImages_GetBackendTextureFromImage(
    const SkImage* self,
    bool flushPendingGrContextIO,
    GrSurfaceOrigin* origin)
{
    auto texture = new GrBackendTexture();
    // TODO: Might need to check the return value, only the validity of the texture is checked by the caller.
    SkImages::GetBackendTextureFromImage(self, texture, flushPendingGrContextIO, origin);
    return texture;
}

extern "C" SkImage* C_SkImages_SubsetTextureFrom(
    GrDirectContext* context,
    const SkImage* image,
    const SkIRect* subset) 
{
    return SkImages::SubsetTextureFrom(context, image, *subset).release();
}

//
// gpu/ganesh/SkSurfaceGanesh.h
//

extern "C" SkSurface* C_SkSurfaces_RenderTarget(
    GrRecordingContext* context,
    skgpu::Budgeted budgeted,
    const SkImageInfo* imageInfo,
    int sampleCount,
    GrSurfaceOrigin surfaceOrigin,
    const SkSurfaceProps* surfaceProps,
    bool shouldCreateWithMips,
    bool isProtected) {
    return SkSurfaces::RenderTarget(
            context,
            budgeted,
            *imageInfo,
            sampleCount,
            surfaceOrigin,
            surfaceProps,
            shouldCreateWithMips, 
            isProtected).release();
}

extern "C" SkSurface* C_SkSurfaces_WrapBackendTexture(
        GrRecordingContext* context,
        const GrBackendTexture* backendTexture,
        GrSurfaceOrigin origin,
        int sampleCnt,
        SkColorType colorType,
        SkColorSpace* colorSpace,
        const SkSurfaceProps* surfaceProps) {
    return SkSurfaces::WrapBackendTexture(
            context,
            *backendTexture,
            origin,
            sampleCnt,
            colorType,
            sp(colorSpace), surfaceProps).release();
}

extern "C" SkSurface* C_SkSurfaces_WrapBackendRenderTarget(
    GrRecordingContext* context,
    const GrBackendRenderTarget* backendRenderTarget,
    GrSurfaceOrigin origin,
    SkColorType colorType,
    SkColorSpace* colorSpace,
    const SkSurfaceProps* surfaceProps
    )
{
    return SkSurfaces::WrapBackendRenderTarget(
            context,
            *backendRenderTarget,
            origin,
            colorType,
            sp(colorSpace),
            surfaceProps).release();
}

extern "C" GrBackendTexture* C_SkSurfaces_GetBackendTexture(
    SkSurface* surface,
    SkSurface::BackendHandleAccess handleAccess) 
{
    return new GrBackendTexture(SkSurfaces::GetBackendTexture(surface, handleAccess));
}

extern "C" void C_SkSurfaces_GetBackendRenderTarget(
    SkSurface* surface,
    SkSurface::BackendHandleAccess handleAccess,
    GrBackendRenderTarget *backendRenderTarget)
{
    *backendRenderTarget = SkSurfaces::GetBackendRenderTarget(surface, handleAccess);
}

extern "C" void C_SkSurfaces_ResolveMSAA(SkSurface* surface)
{
    SkSurfaces::ResolveMSAA(surface);
}
