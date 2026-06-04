#include "../pal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int64_t read_proc_value(const char *key) {
    FILE *f = fopen("/proc/meminfo", "r");
    if (!f) return 0;
    char line[256];
    int64_t value = 0;
    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, key, strlen(key)) == 0) {
            sscanf(line + strlen(key), "%lld", (long long *)&value);
            break;
        }
    }
    fclose(f);
    return value;
}

int64_t pal_mem_used(void) {
    int64_t total = pal_mem_total();
    int64_t free = pal_mem_free();
    return total - free;
}

int64_t pal_mem_total(void) { return read_proc_value("MemTotal:"); }
int64_t pal_mem_free(void) { return read_proc_value("MemFree:"); }
int64_t pal_mem_available(void) { return read_proc_value("MemAvailable:"); }

int64_t pal_mem_percent(void) {
    int64_t total = pal_mem_total();
    if (total <= 0) return 0;
    return pal_mem_used() * 100 / total;
}

int64_t pal_mem_process_bytes(void) {
    FILE *f = fopen("/proc/self/status", "r");
    if (!f) return 0;
    char line[256];
    int64_t bytes = 0;
    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, "VmRSS:", 6) == 0) {
            sscanf(line + 6, "%lld", (long long *)&bytes);
            bytes *= 1024;
            break;
        }
    }
    fclose(f);
    return bytes;
}

void *pal_mem_snapshot(void) {
    extern void *rt_dict_set_i64(void *dict_ptr, int64_t key_kind, int64_t value_kind,
                                  int64_t key_i64, void *key_ptr, int64_t value);
    void *dict = NULL;
    dict = rt_dict_set_i64(dict, 3, 1, 0, "total", pal_mem_total());
    dict = rt_dict_set_i64(dict, 3, 1, 0, "free", pal_mem_free());
    dict = rt_dict_set_i64(dict, 3, 1, 0, "available", pal_mem_available());
    dict = rt_dict_set_i64(dict, 3, 1, 0, "used", pal_mem_used());
    dict = rt_dict_set_i64(dict, 3, 1, 0, "percent", pal_mem_percent());
    return dict;
}

char *pal_mem_format(int64_t bytes) {
    extern char *rt_managed_printf_i64(const char *fmt, long long value);
    if (bytes < 1024) return rt_managed_printf_i64("%lld B", (long long)bytes);
    if (bytes < 1024 * 1024) return rt_managed_printf_i64("%.1f KiB", (double)bytes / 1024.0);
    if (bytes < 1024LL * 1024 * 1024)
        return rt_managed_printf_i64("%.1f MiB", (double)bytes / (1024.0 * 1024.0));
    return rt_managed_printf_i64("%.1f GiB", (double)bytes / (1024.0 * 1024.0 * 1024.0));
}
