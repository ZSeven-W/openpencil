#ifndef OpenPencilPlayer_Bridging_Header_h
#define OpenPencilPlayer_Bridging_Header_h

#include "op_engine.h"

#import <UIKit/UIKit.h>

// Keep the fold API weak-linked and allow builds with earlier iOS SDKs.
NS_INLINE NSArray<NSValue *> * _Nonnull OPActiveDisplayDivisions(UIView * _Nonnull view) {
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 270100
    if (@available(iOS 27.1, *)) {
        NSMutableArray<NSValue *> *frames = [NSMutableArray array];
        for (UIViewReservedRegion *region in
             [view reservedRegionsOfKind:UIViewReservedRegionKind.divisionRegionKind]) {
            [frames addObject:[NSValue valueWithCGRect:region.frame]];
        }
        return frames;
    }
#endif
    return @[];
}


#endif
