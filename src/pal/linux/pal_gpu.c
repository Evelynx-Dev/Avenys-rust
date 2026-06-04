#include "../pal.h"
#include "runtime.h"
#include <stdlib.h>
#include <string.h>

char *pal_gpu_snapshot(void) {
    return rt_managed_from_slice("available=false", 15);
}
