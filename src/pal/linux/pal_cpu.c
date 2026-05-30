#include "../pal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <stdatomic.h>
#include <ctype.h>

static int64_t clock_ns(clockid_t clock_id) {
    struct timespec ts;
    if (clock_gettime(clock_id, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
}

static double cpu_mhz(void) {
    static atomic_int initialized = 0;
    static _Atomic(double) cached = 0.0;
    if (atomic_load_explicit(&initialized, memory_order_acquire))
        return atomic_load_explicit(&cached, memory_order_relaxed);
    double local = 0.0;
    FILE *fh = fopen("/proc/cpuinfo", "r");
    if (fh) {
        char line[256];
        while (fgets(line, sizeof(line), fh)) {
            for (char *p = line; *p; p++) *p = (char)tolower((unsigned char)*p);
            if (strncmp(line, "cpu mhz", 7) == 0) {
                char *colon = strchr(line, ':');
                if (colon) local = strtod(colon + 1, NULL);
                break;
            }
        }
        fclose(fh);
    }
    int expected = 0;
    if (atomic_compare_exchange_strong_explicit(&initialized, &expected, 1,
            memory_order_acq_rel, memory_order_acquire))
        atomic_store_explicit(&cached, local, memory_order_release);
    return atomic_load_explicit(&cached, memory_order_relaxed);
}

int64_t pal_cpu_time_ns(void) { return clock_ns(CLOCK_PROCESS_CPUTIME_ID); }
int64_t pal_cpu_time_ms(void) { return clock_ns(CLOCK_PROCESS_CPUTIME_ID) / 1000000; }
int64_t pal_cpu_mark(void) { return clock_ns(CLOCK_PROCESS_CPUTIME_ID); }

int64_t pal_cpu_elapsed_ms(int64_t start_ns) {
    return (clock_ns(CLOCK_PROCESS_CPUTIME_ID) - start_ns) / 1000000;
}

int64_t pal_cpu_elapsed_ns(int64_t start_ns) {
    return clock_ns(CLOCK_PROCESS_CPUTIME_ID) - start_ns;
}

int64_t pal_cpu_count(void) {
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n > 0 ? (int64_t)n : 1;
}

int64_t pal_cpu_freq_mhz(void) { return (int64_t)cpu_mhz(); }

int64_t pal_cpu_cycles_est(int64_t start_ns) {
    int64_t elapsed = pal_cpu_elapsed_ns(start_ns);
    double mhz = cpu_mhz();
    if (mhz <= 0) return elapsed;
    return (int64_t)((double)elapsed * mhz / 1000.0);
}

void *pal_cpu_loadavg(void) {
    extern void *rt_list_create(int64_t initial_cap, int64_t elem_size);
    extern void *rt_list_push_i64(void *list_ptr, int64_t value);
    void *list = rt_list_create(3, 8);
    FILE *fh = fopen("/proc/loadavg", "r");
    if (fh) {
        double d1, d5, d15;
        if (fscanf(fh, "%lf %lf %lf", &d1, &d5, &d15) == 3) {
            list = rt_list_push_i64(list, (int64_t)(d1 * 100));
            list = rt_list_push_i64(list, (int64_t)(d5 * 100));
            list = rt_list_push_i64(list, (int64_t)(d15 * 100));
        }
        fclose(fh);
    }
    return list;
}

void *pal_cpu_snapshot(void) {
    extern void *rt_list_create(int64_t initial_cap, int64_t elem_size);
    extern void *rt_dict_set_i64(void *dict_ptr, int64_t key_kind, int64_t value_kind,
                                  int64_t key_i64, void *key_ptr, int64_t value);
    void *dict = NULL;
    dict = rt_dict_set_i64(dict, 3, 1, 0, "count", pal_cpu_count());
    dict = rt_dict_set_i64(dict, 3, 1, 0, "freq_mhz", pal_cpu_freq_mhz());
    return dict;
}
