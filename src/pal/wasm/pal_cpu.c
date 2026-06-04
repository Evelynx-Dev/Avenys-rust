// WASM PAL — CPU stubs
// WASM has limited CPU introspection. These return defaults.

#include "pal.h"
#include <time.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int64_t pal_cpu_time_ns(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * 1000000000 + (int64_t)ts.tv_nsec;
}

int64_t pal_cpu_time_ms(void) {
    return pal_cpu_time_ns() / 1000000;
}

int64_t pal_cpu_mark(void) {
    return pal_time_mark();
}

int64_t pal_cpu_elapsed_ms(int64_t start_ns) {
    return pal_time_elapsed_ms(start_ns);
}

int64_t pal_cpu_elapsed_ns(int64_t start_ns) {
    return pal_time_elapsed_ns(start_ns);
}

int64_t pal_cpu_count(void) {
    return 1;
}

int64_t pal_cpu_freq_mhz(void) {
    return 0;
}

int64_t pal_cpu_cycles_est(int64_t start_ns) {
    (void)start_ns;
    return 0;
}

void *pal_cpu_loadavg(void) {
    return NULL;
}

void *pal_cpu_snapshot(void) {
    return NULL;
}
