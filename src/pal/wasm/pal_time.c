// WASM PAL — Time
// Uses clock_gettime with CLOCK_MONOTONIC (available in WASM via WASI).

#include "pal.h"
#include <time.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int64_t pal_time_unix_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) != 0) return -1;
    return (int64_t)ts.tv_sec * 1000 + (int64_t)ts.tv_nsec / 1000000;
}

int64_t pal_time_unix_ns(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) != 0) return -1;
    return (int64_t)ts.tv_sec * 1000000000 + (int64_t)ts.tv_nsec;
}

int64_t pal_time_since_ms(int64_t start_ns) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return -1;
    int64_t now = (int64_t)ts.tv_sec * 1000000000 + (int64_t)ts.tv_nsec;
    return (now - start_ns) / 1000000;
}

int64_t pal_time_since_ns(int64_t start_ns) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return -1;
    int64_t now = (int64_t)ts.tv_sec * 1000000000 + (int64_t)ts.tv_nsec;
    return now - start_ns;
}

void pal_time_sleep_ms(int64_t ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000;
    nanosleep(&ts, NULL);
}

void pal_time_sleep_ns(int64_t ns) {
    struct timespec ts;
    ts.tv_sec = ns / 1000000000;
    ts.tv_nsec = ns % 1000000000;
    nanosleep(&ts, NULL);
}

int64_t pal_time_mark(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return -1;
    return (int64_t)ts.tv_sec * 1000000000 + (int64_t)ts.tv_nsec;
}

int64_t pal_time_elapsed_ms(int64_t start_ns) {
    return pal_time_since_ms(start_ns);
}

int64_t pal_time_elapsed_ns(int64_t start_ns) {
    return pal_time_since_ns(start_ns);
}
