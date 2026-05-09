#include "bindings.h"

#include "include/gpu/ganesh/GrBackendSurface.h"
#include "include/gpu/ganesh/GrBackendSemaphore.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/d3d/GrD3DBackendContext.h"
#include "include/gpu/ganesh/d3d/GrD3DBackendSemaphore.h"
#include "include/gpu/ganesh/d3d/GrD3DBackendSurface.h"
#include "include/gpu/ganesh/d3d/GrD3DDirectContext.h"

// Additional types not yet referenced.
extern "C" void C_GrD3DTypes(GrD3DSurfaceInfo *) {};

//
// gpu/d3d/GrD3DTypes.h
//

extern "C" void C_GrD3DTextureResourceInfo_Construct(GrD3DTextureResourceInfo* uninitialized) {
    new(uninitialized) GrD3DTextureResourceInfo();
}

extern "C" void C_GrD3DFenceInfo_Construct(GrD3DFenceInfo* uninitialized) {
    new(uninitialized) GrD3DFenceInfo();
}

//
// gpu/d3d/GrD3DBackendSemaphore.h
//

extern "C" void C_GrBackendSemaphore_ConstructD3D(
    GrBackendSemaphore* uninitialized,
    const GrD3DFenceInfo* info) {
    new(uninitialized) GrBackendSemaphore(GrBackendSemaphores::MakeD3D(*info));
}

extern "C" void C_GrBackendSemaphores_GetD3DFenceInfo(
    const GrBackendSemaphore* semaphore,
    GrD3DFenceInfo* info) {
    *info = GrBackendSemaphores::GetD3DFenceInfo(*semaphore);
}

//
// gpu/GrBackendSurface.h
//

extern "C" void C_GrBackendFormat_ConstructD3D(GrBackendFormat* uninitialized, DXGI_FORMAT format) {
    new(uninitialized)GrBackendFormat(GrBackendFormats::MakeD3D(format));
}

extern "C" bool C_GrBackendFormats_AsDxgiFormat(const GrBackendFormat* format, DXGI_FORMAT* dxgiFormat) {
    const auto result = GrBackendFormats::AsDxgiFormat(*format);
    if (result == DXGI_FORMAT_UNKNOWN) {
        return false;
    }
    *dxgiFormat = result;
    return true;
}

extern "C" GrBackendTexture* C_GrBackendTexture_newD3D(
    int width, int height,
    const GrD3DTextureResourceInfo* resourceInfo, 
    const char* label,
    size_t labelCount) {
    return new GrBackendTexture(
            GrBackendTextures::MakeD3D(width, height, *resourceInfo, std::string_view(label, labelCount)));
}

extern "C" bool C_GrBackendTextures_GetD3DTextureResourceInfo(
        const GrBackendTexture* texture,
        GrD3DTextureResourceInfo* info) {
    auto result = GrBackendTextures::GetD3DTextureResourceInfo(*texture);
    if (result.fResource.get() == nullptr) {
        return false;
    }
    *info = result;
    return true;
}

extern "C" void C_GrBackendTextures_SetD3DResourceState(
        GrBackendTexture* texture,
        GrD3DResourceStateEnum state) {
    GrBackendTextures::SetD3DResourceState(texture, state);
}

extern "C" void C_GrBackendRenderTarget_ConstructD3D(GrBackendRenderTarget* uninitialized, int width, int height, const GrD3DTextureResourceInfo* resourceInfo) {
    new(uninitialized)GrBackendRenderTarget(GrBackendRenderTargets::MakeD3D(width, height, *resourceInfo));
}

extern "C" bool C_GrBackendRenderTargets_GetD3DTextureResourceInfo(
        const GrBackendRenderTarget* renderTarget,
        GrD3DTextureResourceInfo* info) {
    auto result = GrBackendRenderTargets::GetD3DTextureResourceInfo(*renderTarget);
    if (result.fResource.get() == nullptr) {
        return false;
    }
    *info = result;
    return true;
}

extern "C" void C_GrBackendRenderTargets_SetD3DResourceState(
        GrBackendRenderTarget* renderTarget,
        GrD3DResourceStateEnum state) {
    GrBackendRenderTargets::SetD3DResourceState(renderTarget, state);
}

//
// gpu/GrDirectContext.h
//

extern "C" GrDirectContext* C_GrDirectContexts_MakeD3D(
    const GrD3DBackendContext* backendContext,
    const GrContextOptions* options) {
    if (options) {
        return GrDirectContexts::MakeD3D(*backendContext, *options).release();
    }
    return GrDirectContexts::MakeD3D(*backendContext).release();
}

extern "C" GrDirectContext* C_GrDirectContext_MakeD3D(
    const GrD3DBackendContext* backendContext,
    const GrContextOptions* options) {
    return C_GrDirectContexts_MakeD3D(backendContext, options);
}
