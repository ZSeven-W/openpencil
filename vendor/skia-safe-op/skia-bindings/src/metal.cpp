#ifndef SK_METAL
    #define SK_METAL
#endif

#include "bindings.h"

#include "include/core/SkColorSpace.h"
#include "include/core/SkSurface.h"
#include "include/gpu/ganesh/GrBackendSurface.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/mtl/SkSurfaceMetal.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendContext.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendSurface.h"
#include "include/gpu/ganesh/mtl/GrMtlDirectContext.h"

extern "C" void C_GrMtlTypes(GrMTLTextureUsage*, GrMtlSurfaceInfo *) {};

//
// gpu/ganesh/mtl/GrMtlBackendSurface.h
//

extern "C" void C_GrBackendFormats_ConstructMtl(GrBackendFormat* uninitialized, GrMTLPixelFormat format) {
    new(uninitialized)GrBackendFormat(GrBackendFormats::MakeMtl(format));
}

extern "C" GrMTLPixelFormat C_GrBackendFormats_AsMtlFormat(const GrBackendFormat* backendFormat) {
    return GrBackendFormats::AsMtlFormat(*backendFormat);
}

extern "C" GrBackendTexture* C_GrBackendTextures_newMtl(
    int width, int height,
    skgpu::Mipmapped mipMapped,
    const GrMtlTextureInfo* mtlInfo,
    const char* label,
    size_t labelCount) {
    return new GrBackendTexture(GrBackendTextures::MakeMtl(width, height, mipMapped, *mtlInfo, std::string_view(label, labelCount)));
}

extern "C" bool C_GrBackendTextures_GetMtlTextureInfo(const GrBackendTexture* backendTexture, GrMtlTextureInfo* textureInfo) {
    return GrBackendTextures::GetMtlTextureInfo(*backendTexture, textureInfo);
}

extern "C" void C_GrBackendRenderTargets_ConstructMtl(GrBackendRenderTarget* uninitialized, int width, int height, const GrMtlTextureInfo* mtlInfo) {
    new(uninitialized)GrBackendRenderTarget(GrBackendRenderTargets::MakeMtl(width, height, *mtlInfo));
}

extern "C" bool C_GrBackendRenderTargets_GetMtlTextureInfo(const GrBackendRenderTarget* target, GrMtlTextureInfo* info) {
    return GrBackendRenderTargets::GetMtlTextureInfo(*target, info);
}

//
// gpu/ganesh/mtl/SkSurfaceMetal.h
//

extern "C" SkSurface *C_SkSurfaces_WrapCAMetalLayer(
    GrRecordingContext *context,
    GrMTLHandle layer,
    GrSurfaceOrigin origin,
    int sampleCnt,
    SkColorType colorType,
    SkColorSpace *colorSpace,
    const SkSurfaceProps *surfaceProps,
    GrMTLHandle *drawable) {
    return SkSurfaces::WrapCAMetalLayer(context, layer, origin, sampleCnt, colorType, sp(colorSpace), surfaceProps, drawable).release();
}

extern "C" SkSurface *C_SkSurfaces_WrapMTKView(
    GrRecordingContext *context,
    GrMTLHandle mtkView,
    GrSurfaceOrigin origin,
    int sampleCnt,
    SkColorType colorType,
    SkColorSpace *colorSpace,
    const SkSurfaceProps *surfaceProps) {
    return SkSurfaces::WrapMTKView(context, mtkView, origin, sampleCnt, colorType, sp(colorSpace), surfaceProps).release();
}

//
// gpu/GrDirectContext.h
//

extern "C" GrDirectContext *C_GrContext_MakeMetal(
    const GrMtlBackendContext *context,
    const GrContextOptions *options)
{
    if (options)
    {
        return GrDirectContexts::MakeMetal(*context, *options).release();
    }
    return GrDirectContexts::MakeMetal(*context).release();
}

//
// gpu/mtl/GrMtlBackendContext.h
//

extern "C" void C_GrMtlBackendContext_Construct(
    GrMtlBackendContext* uninitialized,
    const void* device, const void* queue) {
    new (uninitialized) GrMtlBackendContext();
    uninitialized->fDevice.retain(device);
    uninitialized->fQueue.retain(queue);
}

extern "C" void C_GrMtlBackendContext_Destruct(GrMtlBackendContext* self) {
    self->~GrMtlBackendContext();
}

//
// gpu/mtl/GrMtlTypes.h
//

extern "C" void C_GrMtlTextureInfo_Construct(GrMtlTextureInfo* uninitialized, const void* texture) {
    new (uninitialized) GrMtlTextureInfo();
    uninitialized->fTexture.retain(texture);
}

extern "C" void C_GrMtlTextureInfo_Destruct(GrMtlTextureInfo* self) {
    self->~GrMtlTextureInfo();
}

extern "C" bool C_GrMtlTextureInfo_Equals(const GrMtlTextureInfo* lhs, const GrMtlTextureInfo* rhs) {
    return *lhs == *rhs;
}
