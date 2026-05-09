#include "rust_resource_provider.h"

extern "C" bool C_ImageAsset_isMultiFrame(skresources::ImageAsset* self) {
    return self->isMultiFrame();
}

extern "C" skresources::ImageAsset* C_MultiFrameImageAsset_Make(
    SkData* data, skresources::ImageDecodeStrategy decodeStrategy)
{
    return skresources::MultiFrameImageAsset::Make(sp(data), decodeStrategy).release();
}

extern "C" RustResourceProvider* C_RustResourceProvider_New(const RustResourceProvider::Param* param) {
    return new RustResourceProvider(*param);
}
