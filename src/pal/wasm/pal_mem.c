// WASM PAL — Memory stubs
// WASM has no OS-level memory stats. These return reasonable defaults.

#include "pal.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int64_t pal_mem_used(void) { return 0; }
int64_t pal_mem_total(void) { return 0; }
int64_t pal_mem_free(void) { return 0; }
int64_t pal_mem_available(void) { return 0; }
int64_t pal_mem_percent(void) { return 0; }
int64_t pal_mem_process_bytes(void) { return 0; }

void *pal_mem_snapshot(void) {
    return NULL;
}

char *pal_mem_format(int64_t bytes) {
    char buf[64];
    if (bytes < 1024) {
        snprintf(buf, sizeof(buf), "%ld B", (long)bytes);
    } else if (bytes < 1024 * 1024) {
        snprintf(buf, sizeof(buf), "%.1f KB", (double)bytes / 1024);
    } else if (bytes < 1024 * 1024 * 1024) {
        snprintf(buf, sizeof(buf), "%.1f MB", (double)bytes / (1024 * 1024));
    } else {
        snprintf(buf, sizeof(buf), "%.2f GB", (double)bytes / (1024 * 1024 * 1024));
    }
    return strdup(buf);
}
