#include "include/gpu/ganesh/gl/GrGLInterface.h"
#include "include/gpu/ganesh/gl/egl/GrGLMakeEGLInterface.h"

extern "C" const GrGLInterface* C_GrGLInterfaces_MakeEGL() {
    return GrGLInterfaces::MakeEGL().release();
}
