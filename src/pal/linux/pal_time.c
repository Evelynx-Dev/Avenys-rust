#include "../pal.h"
#include <time.h>
#include <unistd.h>

static int64_t clock_ns(clockid_t clock_id) {
    struct timespec ts;
    if (clock_gettime(clock_id, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
}

int64_t pal_time_unix_ms(void) {
    return clock_ns(CLOCK_REALTIME) / 1000000;
}

int64_t pal_time_unix_ns(void) {
    return clock_ns(CLOCK_REALTIME);
}

int64_t pal_time_since_ms(int64_t start_ns) {
    return (clock_ns(CLOCK_REALTIME) - start_ns) / 1000000;
}

int64_t pal_time_since_ns(int64_t start_ns) {
    return clock_ns(CLOCK_REALTIME) - start_ns;
}

void pal_time_sleep_ms(int64_t ms) {
    usleep((useconds_t)(ms * 1000));
}

void pal_time_sleep_ns(int64_t ns) {
    struct timespec ts;
    ts.tv_sec = ns / 1000000000;
    ts.tv_nsec = ns % 1000000000;
    nanosleep(&ts, NULL);
}

int64_t pal_time_mark(void) {
    return clock_ns(CLOCK_MONOTONIC);
}

int64_t pal_time_elapsed_ms(int64_t start_ns) {
    return (clock_ns(CLOCK_MONOTONIC) - start_ns) / 1000000;
}

int64_t pal_time_elapsed_ns(int64_t start_ns) {
    return clock_ns(CLOCK_MONOTONIC) - start_ns;
}
