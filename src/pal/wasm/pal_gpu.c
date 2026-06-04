// WASM PAL — GPU stub
// WASM has no direct GPU access via WebGPU/WebGL from C.

#include "pal.h"

char *pal_gpu_snapshot(void) {
    return NULL;
}
