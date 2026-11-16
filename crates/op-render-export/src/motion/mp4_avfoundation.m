#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

@interface OPMotionWriter : NSObject
@property(strong) AVAssetWriter *writer;
@property(strong) AVAssetWriterInput *input;
@property(strong) AVAssetWriterInputPixelBufferAdaptor *adaptor;
@property int32_t fps;
@end

@implementation OPMotionWriter
@end

static void copy_error(char *out, size_t capacity, NSString *message) {
    if (capacity != 0) {
        snprintf(out, capacity, "%s", (message ?: @"AVFoundation export failed").UTF8String);
    }
}

void *op_motion_mp4_open(const char *path, int width, int height, int fps, char *error, size_t capacity) {
    @autoreleasepool {
        NSString *file = [[NSString alloc] initWithUTF8String:path];
        if (!file) {
            copy_error(error, capacity, @"Invalid output path");
            return NULL;
        }
        NSError *problem = nil;
        AVAssetWriter *writer = [[AVAssetWriter alloc] initWithURL:[NSURL fileURLWithPath:file]
            fileType:AVFileTypeMPEG4 error:&problem];
        if (!writer) {
            copy_error(error, capacity, problem.localizedDescription);
            return NULL;
        }
        NSDictionary *settings = @{
            AVVideoCodecKey: AVVideoCodecTypeH264,
            AVVideoWidthKey: @(width),
            AVVideoHeightKey: @(height),
            AVVideoCompressionPropertiesKey: @{
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
                AVVideoExpectedSourceFrameRateKey: @(fps),
                AVVideoAllowFrameReorderingKey: @NO,
            }
        };
        if (![writer canApplyOutputSettings:settings forMediaType:AVMediaTypeVideo]) {
            copy_error(error, capacity, @"The system H.264 encoder does not support these settings");
            return NULL;
        }
        AVAssetWriterInput *input = [AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeVideo
            outputSettings:settings];
        if (!input || ![writer canAddInput:input]) {
            copy_error(error, capacity, @"Could not add an H.264 input to the MP4 file");
            return NULL;
        }
        AVAssetWriterInputPixelBufferAdaptor *adaptor =
            [AVAssetWriterInputPixelBufferAdaptor assetWriterInputPixelBufferAdaptorWithAssetWriterInput:input
                sourcePixelBufferAttributes:@{
                    (__bridge NSString *)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_32BGRA),
                    (__bridge NSString *)kCVPixelBufferWidthKey: @(width),
                    (__bridge NSString *)kCVPixelBufferHeightKey: @(height),
                    (__bridge NSString *)kCVPixelBufferIOSurfacePropertiesKey: @{},
                }];
        [writer addInput:input];
        writer.shouldOptimizeForNetworkUse = YES;
        if (![writer startWriting]) {
            copy_error(error, capacity, writer.error.localizedDescription);
            return NULL;
        }
        [writer startSessionAtSourceTime:kCMTimeZero];
        OPMotionWriter *motion = [OPMotionWriter new];
        motion.writer = writer;
        motion.input = input;
        motion.adaptor = adaptor;
        motion.fps = fps;
        return (__bridge_retained void *)motion;
    }
}

bool op_motion_mp4_push(void *handle, const uint8_t *pixels, int64_t index, char *error, size_t capacity) {
    @autoreleasepool {
        OPMotionWriter *motion = (__bridge OPMotionWriter *)handle;
        NSTimeInterval deadline = NSProcessInfo.processInfo.systemUptime + 30.0;
        while (!motion.input.readyForMoreMediaData) {
            if (motion.writer.status != AVAssetWriterStatusWriting ||
                NSProcessInfo.processInfo.systemUptime >= deadline) {
                copy_error(error, capacity, motion.writer.error.localizedDescription ?: @"Timed out waiting for the H.264 encoder");
                return false;
            }
            [NSThread sleepForTimeInterval:0.002];
        }
        CVPixelBufferRef buffer = NULL;
        CVPixelBufferPoolRef pool = motion.adaptor.pixelBufferPool;
        if (!pool || CVPixelBufferPoolCreatePixelBuffer(NULL, pool, &buffer) != kCVReturnSuccess) {
            copy_error(error, capacity, @"Could not allocate the video pixel buffer");
            return false;
        }
        if (CVPixelBufferLockBaseAddress(buffer, 0) != kCVReturnSuccess) {
            CVPixelBufferRelease(buffer);
            copy_error(error, capacity, @"Could not lock the video pixel buffer");
            return false;
        }
        uint8_t *out = CVPixelBufferGetBaseAddress(buffer);
        size_t stride = CVPixelBufferGetBytesPerRow(buffer);
        size_t row_size = CVPixelBufferGetWidth(buffer) * 4;
        for (size_t row = 0; row < CVPixelBufferGetHeight(buffer); row++) {
            memcpy(out + stride * row, pixels + row_size * row, row_size);
        }
        CVPixelBufferUnlockBaseAddress(buffer, 0);
        bool ok = [motion.adaptor appendPixelBuffer:buffer withPresentationTime:CMTimeMake(index, motion.fps)];
        CVPixelBufferRelease(buffer);
        if (!ok) {
            copy_error(error, capacity, motion.writer.error.localizedDescription);
        }
        return ok;
    }
}

bool op_motion_mp4_finish(void *handle, int64_t frames, char *error, size_t capacity) {
    @autoreleasepool {
        OPMotionWriter *motion = (__bridge OPMotionWriter *)handle;
        [motion.writer endSessionAtSourceTime:CMTimeMake(frames, motion.fps)];
        [motion.input markAsFinished];
        dispatch_semaphore_t done = dispatch_semaphore_create(0);
        [motion.writer finishWritingWithCompletionHandler:^{ dispatch_semaphore_signal(done); }];
        if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 30LL * NSEC_PER_SEC)) != 0) {
            copy_error(error, capacity, @"Timed out finishing the MP4 file");
            return false;
        }
        if (motion.writer.status != AVAssetWriterStatusCompleted) {
            copy_error(error, capacity, motion.writer.error.localizedDescription);
            return false;
        }
        return true;
    }
}

void op_motion_mp4_close(void *handle) {
    @autoreleasepool {
        OPMotionWriter *motion = (__bridge_transfer OPMotionWriter *)handle;
        if (motion.writer.status == AVAssetWriterStatusWriting) {
            [motion.writer cancelWriting];
        }
    }
}
