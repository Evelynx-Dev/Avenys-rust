#include <ctype.h>
#include <stddef.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <dirent.h>
#include <signal.h>
#include <errno.h>
#include <math.h>

// Fast list implementation - inline storage
// Format: [capacity, length, data...]
// This avoids pointer arithmetic and extra allocations

void *mire_list_create(int64_t initial_cap, int64_t elem_size) {
    if (initial_cap < 4) initial_cap = 4;
    int64_t *ptr = (int64_t *)malloc(16 + initial_cap * elem_size);
    if (!ptr) return NULL;
    ptr[0] = initial_cap;
    ptr[1] = 0;
    return ptr + 1;
}

static inline int64_t mire_list_len(void *list_ptr) {
    if (!list_ptr) return 0;
    return ((int64_t *)list_ptr)[0];
}

static inline int64_t mire_list_cap(void *list_ptr) {
    if (!list_ptr) return 0;
    return ((int64_t *)list_ptr)[-1];
}

static inline void *mire_list_grow(void *list_ptr, int64_t elem_size) {
    int64_t old_cap = mire_list_cap(list_ptr);
    int64_t old_len = mire_list_len(list_ptr);
    int64_t new_cap = old_cap < 4 ? 4 : old_cap + (old_cap >> 1);  // 1.5x growth
    
    int64_t *old_ptr = ((int64_t *)list_ptr) - 1;
    int64_t *new_ptr = (int64_t *)realloc(old_ptr, 16 + new_cap * elem_size);
    if (!new_ptr) return list_ptr;
    
    new_ptr[0] = new_cap;
    new_ptr[1] = old_len;
    return new_ptr + 1;
}

void *mire_list_push_i64(void *list_ptr, int64_t value) {
    if (!list_ptr) {
        list_ptr = mire_list_create(4, 8);
        if (!list_ptr) return NULL;
    }
    
    int64_t len = mire_list_len(list_ptr);
    int64_t cap = mire_list_cap(list_ptr);
    
    if (len >= cap) {
        list_ptr = mire_list_grow(list_ptr, 8);
    }
    
    ((int64_t *)list_ptr)[len + 1] = value;
    ((int64_t *)list_ptr)[0] = len + 1;
    return list_ptr;
}

void *mire_list_push_scalar(void *list_ptr, int64_t value, int64_t elem_size) {
    if (!list_ptr) {
        list_ptr = mire_list_create(4, elem_size > 0 ? elem_size : 8);
        if (!list_ptr) return NULL;
    }
    
    int64_t len = mire_list_len(list_ptr);
    int64_t cap = mire_list_cap(list_ptr);
    
    if (len >= cap) {
        list_ptr = mire_list_grow(list_ptr, elem_size > 0 ? elem_size : 8);
    }
    
    if (elem_size == 8) {
        ((int64_t *)list_ptr)[len + 1] = value;
    } else if (elem_size == 4) {
        *(int32_t *)((char *)list_ptr + 8 + len * 4) = (int32_t)value;
    } else if (elem_size == 2) {
        *(int16_t *)((char *)list_ptr + 8 + len * 2) = (int16_t)value;
    } else if (elem_size == 1) {
        *((int8_t *)list_ptr + 8 + len) = (int8_t)value;
    } else {
        memcpy((char *)list_ptr + 8 + len * elem_size, &value, elem_size);
    }
    
    ((int64_t *)list_ptr)[0] = len + 1;
    return list_ptr;
}

typedef struct MireDictEntry {
    int64_t hash;
    int64_t next;
    int64_t key_i64;
    char *key_str;
} MireDictEntry;

typedef struct {
    int64_t len;
    int64_t cap;
    int64_t key_kind;
    int64_t value_kind;
    int64_t key_size;
    int64_t value_size;
    int64_t bucket_cap;
    int64_t *buckets;
    MireDictEntry *entries;
    uint8_t *key_storage;
    uint8_t *value_storage;
} MireDict;

enum {
    MIRE_KIND_SCALAR = 1,
    MIRE_KIND_BOOL = 2,
    MIRE_KIND_STR = 3,
    MIRE_KIND_MAP = 4,
    MIRE_KIND_PTR = 5,
};

char *mire_dict_to_string(void *dict_ptr);
void *mire_list_push_scalar(void *list_ptr, int64_t value, int64_t elem_size);

typedef struct {
    size_t len;
    size_t cap;
    char data[];
} MireManagedString;

typedef struct MireManagedStringNode {
    char *data_ptr;
    struct MireManagedStringNode *next;
} MireManagedStringNode;

static MireManagedStringNode *mire_managed_strings = NULL;

static void mire_managed_register(char *data_ptr) {
    if (data_ptr == NULL) {
        return;
    }

    MireManagedStringNode *node =
        (MireManagedStringNode *)malloc(sizeof(MireManagedStringNode));
    if (node == NULL) {
        return;
    }
    node->data_ptr = data_ptr;
    node->next = mire_managed_strings;
    mire_managed_strings = node;
}

static void mire_managed_unregister(char *data_ptr) {
    MireManagedStringNode **cursor = &mire_managed_strings;
    while (*cursor != NULL) {
        if ((*cursor)->data_ptr == data_ptr) {
            MireManagedStringNode *node = *cursor;
            *cursor = node->next;
            free(node);
            return;
        }
        cursor = &(*cursor)->next;
    }
}

static int mire_managed_contains(const char *data_ptr) {
    for (MireManagedStringNode *node = mire_managed_strings; node != NULL; node = node->next) {
        if (node->data_ptr == data_ptr) {
            return 1;
        }
    }
    return 0;
}

static size_t mire_string_growth_cap(size_t min_cap) {
    size_t cap = 16;
    while (cap < min_cap) {
        cap += cap >> 1;
    }
    return cap;
}

static MireManagedString *mire_string_header(char *value) {
    if (value == NULL) {
        return NULL;
    }
    return (MireManagedString *)((char *)value - offsetof(MireManagedString, data));
}

static char *mire_strdup_raw(const char *src) {
    size_t len = strlen(src) + 1;
    char *out = (char *)malloc(len);
    if (out == NULL) {
        return NULL;
    }
    memcpy(out, src, len);
    return out;
}

static char *mire_strdup_raw_n(const char *src, size_t len) {
    char *out = (char *)malloc(len + 1);
    if (out == NULL) {
        return NULL;
    }
    if (len > 0) {
        memcpy(out, src, len);
    }
    out[len] = '\0';
    return out;
}

static char *mire_managed_alloc(size_t len) {
    size_t cap = mire_string_growth_cap(len);
    MireManagedString *header =
        (MireManagedString *)malloc(sizeof(MireManagedString) + cap + 1);
    if (header == NULL) {
        return NULL;
    }
    header->len = len;
    header->cap = cap;
    header->data[len] = '\0';
    mire_managed_register(header->data);
    return header->data;
}

static char *mire_managed_from_slice(const char *src, size_t len) {
    char *out = mire_managed_alloc(len);
    if (out == NULL) {
        return mire_strdup_raw("");
    }
    if (len > 0) {
        memcpy(out, src, len);
    }
    out[len] = '\0';
    return out;
}

static char *mire_managed_printf_i64(const char *fmt, long long value) {
    int needed = snprintf(NULL, 0, fmt, value);
    if (needed < 0) {
        return mire_managed_from_slice("", 0);
    }
    char *out = mire_managed_alloc((size_t)needed);
    if (out == NULL) {
        return mire_managed_from_slice("", 0);
    }
    snprintf(out, (size_t)needed + 1, fmt, value);
    return out;
}

static char *mire_managed_printf_f64(const char *fmt, double value) {
    int needed = snprintf(NULL, 0, fmt, value);
    if (needed < 0) {
        return mire_managed_from_slice("", 0);
    }
    char *out = mire_managed_alloc((size_t)needed);
    if (out == NULL) {
        return mire_managed_from_slice("", 0);
    }
    snprintf(out, (size_t)needed + 1, fmt, value);
    return out;
}

static char *mire_alloc_printf_raw_i64(const char *fmt, long long value) {
    int needed = snprintf(NULL, 0, fmt, value);
    if (needed < 0) {
        return mire_strdup_raw("");
    }
    char *out = (char *)malloc((size_t)needed + 1);
    if (out == NULL) {
        return mire_strdup_raw("");
    }
    snprintf(out, (size_t)needed + 1, fmt, value);
    return out;
}

static int64_t mire_clock_ns(clockid_t clock_id) {
    struct timespec ts;
    if (clock_gettime(clock_id, &ts) != 0) {
        return 0;
    }
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
}

void mire_runtime_panic(const char *message) {
    if (message && *message) {
        fprintf(stderr, "runtime error: %s\n", message);
    } else {
        fprintf(stderr, "runtime error\n");
    }
    fflush(stderr);
    exit(101);
}

static double mire_cpu_mhz(void) {
    static atomic_int initialized = 0;
    static _Atomic(double) cached = 0.0;

    if (atomic_load_explicit(&initialized, memory_order_acquire)) {
        return atomic_load_explicit(&cached, memory_order_relaxed);
    }

    double local_cached = 0.0;
    FILE *fh = fopen("/proc/cpuinfo", "r");
    if (fh != NULL) {
        char line[256];
        while (fgets(line, sizeof(line), fh) != NULL) {
            for (char *p = line; *p != '\0'; ++p) {
                *p = (char)tolower((unsigned char)*p);
            }
            if (strncmp(line, "cpu mhz", 7) == 0) {
                char *colon = strchr(line, ':');
                if (colon != NULL) {
                    local_cached = strtod(colon + 1, NULL);
                }
                break;
            }
        }
        fclose(fh);
    }

    int expected = 0;
    if (atomic_compare_exchange_strong_explicit(
            &initialized, &expected, 1, memory_order_acq_rel, memory_order_acquire)) {
        atomic_store_explicit(&cached, local_cached, memory_order_release);
        return local_cached;
    }

    return atomic_load_explicit(&cached, memory_order_relaxed);
}

static uint64_t mire_hash_string(const char *src) {
    uint64_t hash = 1469598103934665603ULL;
    if (src == NULL) {
        return hash;
    }
    while (*src != '\0') {
        hash ^= (uint64_t)(unsigned char)*src;
        hash *= 1099511628211ULL;
        ++src;
    }
    return hash;
}

static uint64_t mire_hash_u64(uint64_t value) {
    value ^= value >> 33;
    value *= 0xff51afd7ed558ccdULL;
    value ^= value >> 33;
    value *= 0xc4ceb9fe1a85ec53ULL;
    value ^= value >> 33;
    return value;
}

static uint64_t mire_hash_key(int64_t key_kind, int64_t key_i64, const void *key_ptr) {
    if (key_kind == MIRE_KIND_STR) {
        return mire_hash_string((const char *)key_ptr);
    }
    if (key_kind == MIRE_KIND_MAP || key_kind == MIRE_KIND_PTR) {
        return mire_hash_u64((uint64_t)(uintptr_t)key_ptr);
    }
    return mire_hash_u64((uint64_t)key_i64);
}

static int64_t mire_kind_size(int64_t kind) {
    switch (kind) {
        case MIRE_KIND_BOOL:
            return 1;
        case MIRE_KIND_STR:
        case MIRE_KIND_MAP:
        case MIRE_KIND_PTR:
            return 8;
        default:
            return 8;
    }
}

static void *mire_dict_key_slot(MireDict *dict, int64_t index) {
    return dict->key_storage + index * dict->key_size;
}

static void *mire_dict_value_slot(MireDict *dict, int64_t index) {
    return dict->value_storage + index * dict->value_size;
}

static void mire_dict_write_scalar(void *slot, int64_t size, int64_t value) {
    switch (size) {
        case 1:
            *(uint8_t *)slot = (uint8_t)value;
            break;
        case 2:
            *(uint16_t *)slot = (uint16_t)value;
            break;
        case 4:
            *(uint32_t *)slot = (uint32_t)value;
            break;
        default:
            *(int64_t *)slot = value;
            break;
    }
}

static int mire_dict_key_equals(
    const MireDict *dict,
    int64_t entry_index,
    int64_t key_i64,
    const void *key_ptr
) {
    const void *slot = dict->key_storage + entry_index * dict->key_size;
    if (dict->key_kind == MIRE_KIND_STR) {
        const char *stored = *(const char **)slot;
        return strcmp(stored, (const char *)key_ptr) == 0;
    }
    if (dict->key_kind == MIRE_KIND_MAP || dict->key_kind == MIRE_KIND_PTR) {
        const void *stored = *(const void **)slot;
        return stored == key_ptr;
    }
    int64_t stored = 0;
    int64_t size = dict->key_size;
    switch (size) {
        case 1:
            stored = *(const uint8_t *)slot;
            break;
        case 2:
            stored = *(const uint16_t *)slot;
            break;
        case 4:
            stored = *(const uint32_t *)slot;
            break;
        default:
            stored = *(const int64_t *)slot;
            break;
    }
    return stored == key_i64;
}

static void mire_dict_store_key(
    MireDict *dict,
    int64_t entry_index,
    int64_t key_i64,
    const void *key_ptr,
    int replacing
) {
    void *slot = mire_dict_key_slot(dict, entry_index);
    if (dict->key_kind == MIRE_KIND_STR) {
        if (replacing) {
            char *existing = *(char **)slot;
            if (existing != NULL) {
                free(existing);
            }
        }
        const char *src = (const char *)key_ptr;
        char *copy = mire_strdup_raw(src);
        memcpy(slot, &copy, sizeof(char *));
        return;
    }
    if (dict->key_kind == MIRE_KIND_MAP || dict->key_kind == MIRE_KIND_PTR) {
        void *ptr = (void *)key_ptr;
        memcpy(slot, &ptr, sizeof(void *));
        return;
    }
    mire_dict_write_scalar(slot, dict->key_size, key_i64);
}

static void mire_dict_store_value(
    MireDict *dict,
    int64_t entry_index,
    int64_t value_i64,
    const void *value_ptr
) {
    void *slot = mire_dict_value_slot(dict, entry_index);
    if (dict->value_kind == MIRE_KIND_STR) {
        void *ptr = (void *)value_ptr;
        memcpy(slot, &ptr, sizeof(void *));
        return;
    }
    if (dict->value_kind == MIRE_KIND_MAP || dict->value_kind == MIRE_KIND_PTR) {
        void *ptr = (void *)value_ptr;
        memcpy(slot, &ptr, sizeof(void *));
        return;
    }
    mire_dict_write_scalar(slot, dict->value_size, value_i64);
}

static int64_t mire_dict_read_scalar(const MireDict *dict, int64_t entry_index) {
    const void *slot = dict->value_storage + entry_index * dict->value_size;
    switch (dict->value_size) {
        case 1:
            return *(const uint8_t *)slot;
        case 2:
            return *(const uint16_t *)slot;
        case 4:
            return *(const uint32_t *)slot;
        default:
            return *(const int64_t *)slot;
    }
}

static int64_t mire_dict_read_key_scalar(const MireDict *dict, int64_t entry_index) {
    const void *slot = dict->key_storage + entry_index * dict->key_size;
    switch (dict->key_size) {
        case 1:
            return *(const uint8_t *)slot;
        case 2:
            return *(const uint16_t *)slot;
        case 4:
            return *(const uint32_t *)slot;
        default:
            return *(const int64_t *)slot;
    }
}

static void *mire_dict_read_ptr(const MireDict *dict, int64_t entry_index) {
    const void *slot = dict->value_storage + entry_index * dict->value_size;
    return (void *)*(const void **)slot;
}

static void *mire_dict_read_key_ptr(const MireDict *dict, int64_t entry_index) {
    const void *slot = dict->key_storage + entry_index * dict->key_size;
    return (void *)*(const void **)slot;
}

static void mire_dict_clear_buckets(MireDict *dict) {
    if (dict == NULL || dict->buckets == NULL || dict->bucket_cap <= 0) {
        return;
    }
    for (int64_t i = 0; i < dict->bucket_cap; ++i) {
        dict->buckets[i] = -1;
    }
}

static int mire_dict_rehash(MireDict *dict, int64_t bucket_cap) {
    if (dict == NULL) {
        return 0;
    }
    if (bucket_cap < 16) {
        bucket_cap = 16;
    }
    int64_t *buckets = (int64_t *)malloc((size_t)bucket_cap * sizeof(int64_t));
    if (buckets == NULL) {
        return 0;
    }
    free(dict->buckets);
    dict->buckets = buckets;
    dict->bucket_cap = bucket_cap;
    mire_dict_clear_buckets(dict);

    for (int64_t i = 0; i < dict->len; ++i) {
        int64_t bucket = (int64_t)(dict->entries[i].hash & (uint64_t)(dict->bucket_cap - 1));
        dict->entries[i].next = dict->buckets[bucket];
        dict->buckets[bucket] = i;
    }
    return 1;
}

static int mire_dict_resize_storage(MireDict *dict, int64_t new_cap) {
    if (dict == NULL) {
        return 0;
    }
    uint8_t *next_keys = (uint8_t *)realloc(
        dict->key_storage,
        (size_t)new_cap * (size_t)dict->key_size
    );
    if (next_keys == NULL) {
        return 0;
    }
    dict->key_storage = next_keys;
    uint8_t *next_values = (uint8_t *)realloc(
        dict->value_storage,
        (size_t)new_cap * (size_t)dict->value_size
    );
    if (next_values == NULL) {
        return 0;
    }
    dict->value_storage = next_values;
    return 1;
}

static int mire_dict_grow_entries(MireDict *dict) {
    if (dict == NULL) {
        return 0;
    }
    int64_t next_cap = dict->cap == 0 ? 4 : dict->cap * 2;
    MireDictEntry *next_entries = (MireDictEntry *)realloc(
        dict->entries,
        (size_t)next_cap * sizeof(MireDictEntry)
    );
    if (next_entries == NULL) {
        return 0;
    }
    dict->entries = next_entries;
    dict->cap = next_cap;
    if (!mire_dict_resize_storage(dict, next_cap)) {
        return 0;
    }
    return 1;
}

static int mire_dict_maybe_grow_buckets(MireDict *dict) {
    if (dict == NULL) {
        return 0;
    }
    if (dict->bucket_cap == 0) {
        return mire_dict_rehash(dict, 16);
    }
    if ((dict->len + 1) * 2 < dict->bucket_cap) {
        return 1;
    }
    return mire_dict_rehash(dict, dict->bucket_cap * 2);
}

static MireDict *mire_dict_ensure(void *dict_ptr) {
    MireDict *dict = (MireDict *)dict_ptr;
    if (dict != NULL) {
        return dict;
    }

    dict = (MireDict *)calloc(1, sizeof(MireDict));
    if (dict == NULL) {
        return NULL;
    }
    dict->cap = 4;
    dict->key_kind = MIRE_KIND_SCALAR;
    dict->value_kind = MIRE_KIND_SCALAR;
    dict->key_size = 8;
    dict->value_size = 8;
    dict->entries = (MireDictEntry *)calloc((size_t)dict->cap, sizeof(MireDictEntry));
    if (dict->entries == NULL) {
        free(dict);
        return NULL;
    }
    dict->key_storage = (uint8_t *)calloc((size_t)dict->cap, (size_t)dict->key_size);
    dict->value_storage = (uint8_t *)calloc((size_t)dict->cap, (size_t)dict->value_size);
    if (dict->key_storage == NULL || dict->value_storage == NULL) {
        free(dict->key_storage);
        free(dict->value_storage);
        free(dict->entries);
        free(dict);
        return NULL;
    }
    if (!mire_dict_rehash(dict, 16)) {
        free(dict->entries);
        free(dict->key_storage);
        free(dict->value_storage);
        free(dict);
        return NULL;
    }
    return dict;
}

static MireDict *mire_dict_ensure_kind(void *dict_ptr, int64_t key_kind, int64_t value_kind) {
    MireDict *dict = mire_dict_ensure(dict_ptr);
    if (dict == NULL) {
        return NULL;
    }
    if (dict->key_kind == MIRE_KIND_SCALAR) {
        dict->key_kind = key_kind;
    }
    if (dict->value_kind == MIRE_KIND_SCALAR) {
        dict->value_kind = value_kind;
    }
    dict->key_size = mire_kind_size(dict->key_kind);
    dict->value_size = mire_kind_size(dict->value_kind);
    mire_dict_resize_storage(dict, dict->cap);
    return dict;
}

static int64_t mire_dict_find(
    MireDict *dict,
    int64_t key_i64,
    const void *key_ptr,
    uint64_t hash
) {
    if (dict == NULL || dict->buckets == NULL || dict->bucket_cap <= 0) {
        return -1;
    }
    int64_t bucket = (int64_t)(hash & (uint64_t)(dict->bucket_cap - 1));
    int64_t index = dict->buckets[bucket];
    while (index >= 0) {
        MireDictEntry *entry = &dict->entries[index];
        if (entry->hash == hash && mire_dict_key_equals(dict, index, key_i64, key_ptr)) {
            return index;
        }
        index = entry->next;
    }
    return -1;
}

int64_t mire_wall_mark_ns(void) {
    return mire_clock_ns(CLOCK_MONOTONIC);
}

int64_t mire_wall_elapsed_ms(int64_t start_ns) {
    int64_t end_ns = mire_clock_ns(CLOCK_MONOTONIC);
    if (end_ns <= start_ns) {
        return 0;
    }
    return (end_ns - start_ns) / 1000000LL;
}

char *mire_wall_elapsed_ms_str(int64_t start_ns) {
    int64_t end_ns = mire_clock_ns(CLOCK_MONOTONIC);
    if (end_ns <= start_ns) {
        return mire_managed_from_slice("0.000", 5);
    }
    return mire_managed_printf_f64("%.3f", (double)(end_ns - start_ns) / 1000000.0);
}

int64_t mire_cpu_mark_ns(void) {
    return mire_clock_ns(CLOCK_PROCESS_CPUTIME_ID);
}

int64_t mire_cpu_elapsed_ms(int64_t start_ns) {
    int64_t end_ns = mire_clock_ns(CLOCK_PROCESS_CPUTIME_ID);
    if (end_ns <= start_ns) {
        return 0;
    }
    return (end_ns - start_ns) / 1000000LL;
}

char *mire_cpu_elapsed_ms_str(int64_t start_ns) {
    int64_t end_ns = mire_clock_ns(CLOCK_PROCESS_CPUTIME_ID);
    if (end_ns <= start_ns) {
        return mire_managed_from_slice("0.000", 5);
    }
    return mire_managed_printf_f64("%.3f", (double)(end_ns - start_ns) / 1000000.0);
}

int64_t mire_cpu_cycles_est(int64_t start_ns) {
    int64_t end_ns = mire_clock_ns(CLOCK_PROCESS_CPUTIME_ID);
    if (end_ns <= start_ns) {
        return 0;
    }
    double mhz = mire_cpu_mhz();
    if (mhz <= 0.0) {
        return 0;
    }
    double elapsed_ns = (double)(end_ns - start_ns);
    return (int64_t)(elapsed_ns * mhz / 1000.0);
}

int64_t mire_mem_process_bytes(void) {
    FILE *fh = fopen("/proc/self/status", "r");
    if (fh == NULL) {
        return 0;
    }

    char line[256];
    while (fgets(line, sizeof(line), fh) != NULL) {
        if (strncmp(line, "VmRSS:", 6) == 0) {
            long long kb = atoll(line + 6);
            fclose(fh);
            return (int64_t)kb * 1024LL;
        }
    }

    fclose(fh);
    return 0;
}

char *mire_mem_format(int64_t bytes) {
    return mire_managed_printf_i64("%lld B", (long long)bytes);
}

char *mire_gpu_snapshot(void) {
    return mire_managed_from_slice("available=false", 15);
}

char *mire_i64_to_string(int64_t value) {
    return mire_managed_printf_i64("%lld", (long long)value);
}

char *mire_bool_to_string(int64_t value) {
    return mire_managed_from_slice(value ? "true" : "false", value ? 4 : 5);
}

char *mire_f64_to_string(double value) {
    return mire_managed_printf_f64("%.6g", value);
}

char *mire_string_copy(const char *value) {
    if (value == NULL) {
        return mire_managed_from_slice("", 0);
    }
    return mire_managed_from_slice(value, strlen(value));
}

char *mire_string_concat(const char *left, const char *right) {
    if (left == NULL) left = "";
    if (right == NULL) right = "";

    size_t left_len = strlen(left);
    size_t right_len = strlen(right);
    if (left_len > SIZE_MAX - right_len) {
        return mire_managed_from_slice("", 0);
    }
    size_t total_len = left_len + right_len;
    char *result = mire_managed_alloc(total_len);
    if (result == NULL) {
        return mire_managed_from_slice("", 0);
    }
    if (left_len > 0) {
        memcpy(result, left, left_len);
    }
    if (right_len > 0) {
        memcpy(result + left_len, right, right_len);
    }
    result[total_len] = '\0';
    return result;
}

char *mire_string_append_owned(char *value, const char *suffix) {
    if (suffix == NULL || *suffix == '\0') {
        return value;
    }
    if (value == NULL) {
        return mire_string_copy(suffix);
    }
    if (!mire_managed_contains(value)) {
        return mire_string_concat(value, suffix);
    }

    MireManagedString *header = mire_string_header(value);
    size_t suffix_len = strlen(suffix);
    if (header->len > SIZE_MAX - suffix_len) {
        return mire_managed_from_slice("", 0);
    }
    size_t new_len = header->len + suffix_len;
    if (new_len > header->cap) {
        char *previous_data = header->data;
        size_t new_cap = mire_string_growth_cap(new_len);
        header = (MireManagedString *)realloc(
            header,
            sizeof(MireManagedString) + new_cap + 1
        );
        if (header == NULL) {
            return mire_managed_from_slice("", 0);
        }
        if (header->data != previous_data) {
            mire_managed_unregister(previous_data);
            mire_managed_register(header->data);
        }
        header->cap = new_cap;
        value = header->data;
    }

    memcpy(value + header->len, suffix, suffix_len);
    header->len = new_len;
    value[new_len] = '\0';
    return value;
}

char mire_unicode_to_lower(unsigned char c) {
    if (c >= 'A' && c <= 'Z') return c + 32;
    if (c >= 192 && c <= 214) return c + 32;
    if (c >= 216 && c <= 222) return c + 32;
    if (c >= 0xC0 && c <= 0xC6) return c + 32;
    if (c >= 0xC8 && c <= 0xCF) return c + 32;
    if (c >= 0xD0 && c <= 0xD6) return c + 32;
    if (c >= 0xD8 && c <= 0xDE) return c + 32;
    return c;
}

char mire_unicode_to_upper(unsigned char c) {
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 224 && c <= 246) return c - 32;
    if (c >= 248 && c <= 254) return c - 32;
    if (c >= 0xE0 && c <= 0xE6) return c - 32;
    if (c >= 0xE8 && c <= 0xEF) return c - 32;
    if (c >= 0xF0 && c <= 0xF6) return c - 32;
    if (c >= 0xF8 && c <= 0xFE) return c - 32;
    return c;
}

char *mire_string_to_upper(const char *value) {
    if (value == NULL) {
        return mire_managed_from_slice("", 0);
    }
    size_t len = strlen(value);
    char *result = mire_managed_alloc(len);
    if (result == NULL) {
        return mire_managed_from_slice("", 0);
    }
    for (size_t i = 0; i < len; i++) {
        result[i] = mire_unicode_to_upper((unsigned char)value[i]);
    }
    result[len] = '\0';
    return result;
}

char *mire_string_to_lower(const char *value) {
    if (value == NULL) {
        return mire_managed_from_slice("", 0);
    }
    size_t len = strlen(value);
    char *result = mire_managed_alloc(len);
    if (result == NULL) {
        return mire_managed_from_slice("", 0);
    }
    for (size_t i = 0; i < len; i++) {
        result[i] = mire_unicode_to_lower((unsigned char)value[i]);
    }
    result[len] = '\0';
    return result;
}

void mire_string_free(char *value) {
    if (value == NULL || !mire_managed_contains(value)) {
        return;
    }
    mire_managed_unregister(value);
    free(mire_string_header(value));
}

void *mire_list_push_ptr(void *list_ptr, void *value) {
    if (!list_ptr) {
        list_ptr = mire_list_create(4, sizeof(void *));
        if (!list_ptr) return NULL;
    }
    
    int64_t len = mire_list_len(list_ptr);
    int64_t cap = mire_list_cap(list_ptr);
    
    if (len >= cap) {
        list_ptr = mire_list_grow(list_ptr, sizeof(void *));
    }
    
    ((void **)((int64_t *)list_ptr + 1))[len] = value;
    ((int64_t *)list_ptr)[0] = len + 1;
    return list_ptr;
}

static char *mire_dict_format_scalar(int64_t value, int64_t kind) {
    if (kind == MIRE_KIND_BOOL) {
        return mire_strdup_raw(value ? "true" : "false");
    }
    return mire_alloc_printf_raw_i64("%lld", (long long)value);
}

static char *mire_dict_format_key(const MireDict *dict, int64_t entry_index) {
    int64_t kind = dict->key_kind;
    if (kind == MIRE_KIND_STR) {
        const char *src = *(const char **)(dict->key_storage + entry_index * dict->key_size);
        size_t len = strlen(src);
        char *out = (char *)malloc(len + 3);
        if (out == NULL) {
            return mire_strdup_raw("''");
        }
        out[0] = '\'';
        memcpy(out + 1, src, len);
        out[len + 1] = '\'';
        out[len + 2] = '\0';
        return out;
    }
    if (kind == MIRE_KIND_MAP || kind == MIRE_KIND_PTR) {
        return mire_strdup_raw("<ptr>");
    }
    int64_t scalar = mire_dict_read_key_scalar(dict, entry_index);
    return mire_dict_format_scalar(scalar, kind);
}

static char *mire_dict_format_value(const MireDict *dict, int64_t entry_index) {
    int64_t kind = dict->value_kind;
    if (kind == MIRE_KIND_STR) {
        const char *src = *(const char **)(dict->value_storage + entry_index * dict->value_size);
        size_t len = strlen(src);
        char *out = (char *)malloc(len + 3);
        if (out == NULL) {
            return mire_strdup_raw("''");
        }
        out[0] = '\'';
        memcpy(out + 1, src, len);
        out[len + 1] = '\'';
        out[len + 2] = '\0';
        return out;
    }
    if (kind == MIRE_KIND_MAP) {
        char *managed_result = mire_dict_to_string(mire_dict_read_ptr(dict, entry_index));
        if (managed_result == NULL) {
            return mire_strdup_raw("{}");
        }
        size_t len = strlen(managed_result);
        char *out = (char *)malloc(len + 1);
        if (out == NULL) {
            return mire_strdup_raw("{}");
        }
        memcpy(out, managed_result, len + 1);
        return out;
    }
    if (kind == MIRE_KIND_PTR) {
        return mire_strdup_raw("<ptr>");
    }
    int64_t scalar = mire_dict_read_scalar(dict, entry_index);
    return mire_dict_format_scalar(scalar, kind);
}

static void mire_dict_free_repr(char *repr) {
    if (repr == NULL) {
        return;
    }
    if (mire_managed_contains(repr)) {
        mire_string_free(repr);
        return;
    }
    free(repr);
}

int64_t mire_dict_get_i64(
    void *dict_ptr,
    int64_t key_kind,
    int64_t key_i64,
    void *key_ptr,
    int64_t default_value
) {
    MireDict *dict = (MireDict *)dict_ptr;
    uint64_t hash = mire_hash_key(key_kind, key_i64, key_ptr);
    int64_t entry_index = mire_dict_find(dict, key_i64, key_ptr, hash);
    if (entry_index < 0) {
        return default_value;
    }
    return mire_dict_read_scalar(dict, entry_index);
}

void *mire_dict_set_i64(
    void *dict_ptr,
    int64_t key_kind,
    int64_t value_kind,
    int64_t key_i64,
    void *key_ptr,
    int64_t value
) {
    MireDict *dict = mire_dict_ensure_kind(dict_ptr, key_kind, value_kind);
    if (dict == NULL) {
        return dict_ptr;
    }

    uint64_t hash = mire_hash_key(key_kind, key_i64, key_ptr);
    int64_t existing_index = mire_dict_find(dict, key_i64, key_ptr, hash);
    if (existing_index >= 0) {
        mire_dict_store_key(dict, existing_index, key_i64, key_ptr, 1);
        mire_dict_store_value(dict, existing_index, value, NULL);
        return dict;
    }

    if (dict->len == dict->cap && !mire_dict_grow_entries(dict)) {
        return dict;
    }
    if (!mire_dict_maybe_grow_buckets(dict)) {
        return dict;
    }

    int64_t index = dict->len;
    int64_t bucket = (int64_t)(hash & (uint64_t)(dict->bucket_cap - 1));
    dict->entries[index].hash = hash;
    dict->entries[index].next = dict->buckets[bucket];
    dict->buckets[bucket] = index;
    mire_dict_store_key(dict, index, key_i64, key_ptr, 0);
    mire_dict_store_value(dict, index, value, NULL);
    dict->len += 1;
    return dict;
}

void *mire_dict_get_ptr(
    void *dict_ptr,
    int64_t key_kind,
    int64_t key_i64,
    void *key_ptr,
    void *default_value
) {
    MireDict *dict = (MireDict *)dict_ptr;
    uint64_t hash = mire_hash_key(key_kind, key_i64, key_ptr);
    int64_t entry_index = mire_dict_find(dict, key_i64, key_ptr, hash);
    if (entry_index < 0) {
        return default_value;
    }
    return mire_dict_read_ptr(dict, entry_index);
}

void *mire_dict_set_ptr(
    void *dict_ptr,
    int64_t key_kind,
    int64_t value_kind,
    int64_t key_i64,
    void *key_ptr,
    void *value
) {
    MireDict *dict = mire_dict_ensure_kind(dict_ptr, key_kind, value_kind);
    if (dict == NULL) {
        return dict_ptr;
    }

    uint64_t hash = mire_hash_key(key_kind, key_i64, key_ptr);
    int64_t existing_index = mire_dict_find(dict, key_i64, key_ptr, hash);
    if (existing_index >= 0) {
        mire_dict_store_key(dict, existing_index, key_i64, key_ptr, 1);
        mire_dict_store_value(dict, existing_index, 0, value);
        return dict;
    }

    if (dict->len == dict->cap && !mire_dict_grow_entries(dict)) {
        return dict;
    }
    if (!mire_dict_maybe_grow_buckets(dict)) {
        return dict;
    }

    int64_t index = dict->len;
    int64_t bucket = (int64_t)(hash & (uint64_t)(dict->bucket_cap - 1));
    dict->entries[index].hash = hash;
    dict->entries[index].next = dict->buckets[bucket];
    dict->buckets[bucket] = index;
    mire_dict_store_key(dict, index, key_i64, key_ptr, 0);
    mire_dict_store_value(dict, index, 0, value);
    dict->len += 1;
    return dict;
}

char *mire_dict_to_string(void *dict_ptr) {
    MireDict *dict = (MireDict *)dict_ptr;
    if (dict == NULL || dict->len == 0) {
        return mire_managed_from_slice("{}", 2);
    }

    size_t total = 3;
    for (int64_t i = 0; i < dict->len; ++i) {
        char *key_repr = mire_dict_format_key(dict, i);
        char *value_repr = mire_dict_format_value(dict, i);
        total += strlen(key_repr) + strlen(value_repr) + 4;
        mire_dict_free_repr(key_repr);
        mire_dict_free_repr(value_repr);
    }

    char *out = mire_managed_alloc(total - 1);
    if (out == NULL) {
        return mire_managed_from_slice("{}", 2);
    }

    size_t pos = 0;
    out[pos++] = '{';
    for (int64_t i = 0; i < dict->len; ++i) {
        char *key_repr = mire_dict_format_key(dict, i);
        char *value_repr = mire_dict_format_value(dict, i);
        if (i > 0) {
            out[pos++] = ',';
            out[pos++] = ' ';
        }
        size_t key_len = strlen(key_repr);
        memcpy(out + pos, key_repr, key_len);
        pos += key_len;
        out[pos++] = ':';
        out[pos++] = ' ';
        size_t value_len = strlen(value_repr);
        memcpy(out + pos, value_repr, value_len);
        pos += value_len;
        mire_dict_free_repr(key_repr);
        mire_dict_free_repr(value_repr);
    }
    out[pos++] = '}';
    out[pos] = '\0';
    return out;
}

char *mire_strings_replace(const char *input, const char *from, const char *to) {
    if (input == NULL || from == NULL || to == NULL) {
        return mire_managed_from_slice("", 0);
    }

    size_t input_len = strlen(input);
    size_t from_len = strlen(from);
    size_t to_len = strlen(to);
    if (from_len == 0) {
        return mire_managed_from_slice(input, input_len);
    }

    size_t matches = 0;
    const char *cursor = input;
    while ((cursor = strstr(cursor, from)) != NULL) {
        matches += 1;
        cursor += from_len;
    }

    size_t out_len = input_len;
    if (to_len >= from_len) {
        size_t delta = to_len - from_len;
        if (matches > 0 && delta > 0 && matches > (SIZE_MAX - out_len) / delta) {
            return mire_managed_from_slice(input, input_len);
        }
        out_len += matches * delta;
    } else {
        size_t delta = from_len - to_len;
        out_len -= matches * delta;
    }
    char *out = mire_managed_alloc(out_len);
    if (out == NULL) {
        return mire_managed_from_slice(input, input_len);
    }

    const char *src = input;
    char *dst = out;
    while ((cursor = strstr(src, from)) != NULL) {
        size_t chunk = (size_t)(cursor - src);
        memcpy(dst, src, chunk);
        dst += chunk;
        memcpy(dst, to, to_len);
        dst += to_len;
        src = cursor + from_len;
    }

    strcpy(dst, src);
    return out;
}

int64_t mire_strings_contains(const char *input, const char *needle) {
    if (input == NULL || needle == NULL) {
        return 0;
    }
    return strstr(input, needle) != NULL ? 1 : 0;
}

char *mire_strings_substr(const char *input, int64_t start, int64_t length) {
    if (input == NULL) {
        return mire_managed_from_slice("", 0);
    }
    if (start < 0 || length <= 0) {
        return mire_managed_from_slice("", 0);
    }
    size_t input_len = strlen(input);
    size_t start_u = (size_t)start;
    if (start_u >= input_len) {
        return mire_managed_from_slice("", 0);
    }
    size_t len_u = (size_t)length;
    if (len_u > input_len - start_u) {
        len_u = input_len - start_u;
    }
    return mire_managed_from_slice(input + start_u, len_u);
}

char *mire_strings_repeat(const char *input, int64_t times) {
    if (input == NULL || times <= 0) {
        return mire_managed_from_slice("", 0);
    }
    size_t src_len = strlen(input);
    size_t times_u = (size_t)times;
    if (src_len == 0 || times_u == 0) {
        return mire_managed_from_slice("", 0);
    }
    if (times_u > SIZE_MAX / src_len) {
        return mire_managed_from_slice("", 0);
    }
    size_t out_len = src_len * times_u;
    char *out = mire_managed_alloc(out_len);
    if (out == NULL) {
        return mire_managed_from_slice("", 0);
    }
    char *dst = out;
    for (size_t i = 0; i < times_u; ++i) {
        memcpy(dst, input, src_len);
        dst += src_len;
    }
    out[out_len] = '\0';
    return out;
}

static char *mire_strings_pad_core(const char *input, int64_t width, const char *pad, int left_pad) {
    if (input == NULL) {
        input = "";
    }
    if (pad == NULL || *pad == '\0') {
        pad = " ";
    }
    size_t input_len = strlen(input);
    if (width <= 0 || (size_t)width <= input_len) {
        return mire_managed_from_slice(input, input_len);
    }
    size_t pad_len = strlen(pad);
    size_t width_u = (size_t)width;
    size_t fill_len = width_u - input_len;
    char *out = mire_managed_alloc(width_u);
    if (out == NULL) {
        return mire_managed_from_slice(input, input_len);
    }
    size_t fill_written = 0;
    while (fill_written < fill_len) {
        size_t chunk = pad_len;
        if (chunk > fill_len - fill_written) {
            chunk = fill_len - fill_written;
        }
        if (left_pad) {
            memcpy(out + fill_written, pad, chunk);
        } else {
            memcpy(out + input_len + fill_written, pad, chunk);
        }
        fill_written += chunk;
    }
    if (left_pad) {
        memcpy(out + fill_len, input, input_len);
    } else {
        memcpy(out, input, input_len);
    }
    out[width_u] = '\0';
    return out;
}

char *mire_strings_pad_left(const char *input, int64_t width, const char *pad) {
    return mire_strings_pad_core(input, width, pad, 1);
}

char *mire_strings_pad_right(const char *input, int64_t width, const char *pad) {
    return mire_strings_pad_core(input, width, pad, 0);
}

int64_t mire_list_pop_i64(void *list_ptr) {
    if (!list_ptr) return 0;
    int64_t len = mire_list_len(list_ptr);
    if (len <= 0) return 0;
    int64_t value = ((int64_t *)list_ptr)[len];
    ((int64_t *)list_ptr)[0] = len - 1;
    return value;
}

void *mire_list_concat(void *left_ptr, void *right_ptr) {
    if (!left_ptr && !right_ptr) return NULL;
    
    int64_t left_len = mire_list_len(left_ptr);
    int64_t right_len = mire_list_len(right_ptr);
    int64_t total_len = left_len + right_len;
    
    if (total_len == 0) return NULL;
    
    int64_t new_cap = 4;
    while (new_cap < total_len) new_cap += new_cap >> 1;
    
    int64_t *new_base = (int64_t *)malloc(16 + new_cap * 8);
    if (!new_base) return NULL;
    
    new_base[0] = new_cap;
    new_base[1] = total_len;
    int64_t *new_data = new_base + 1;
    
    if (left_ptr && left_len > 0) {
        memcpy(new_data + 1, (int64_t *)left_ptr + 1, (size_t)left_len * 8);
    }
    
    if (right_ptr && right_len > 0) {
        memcpy(new_data + 1 + left_len, (int64_t *)right_ptr + 1, (size_t)right_len * 8);
    }
    
    return new_data;
}

void *mire_list_slice(void *list_ptr, int64_t start, int64_t end) {
    if (!list_ptr) return NULL;
    
    int64_t len = mire_list_len(list_ptr);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end) return NULL;
    
    int64_t new_len = end - start;
    int64_t new_cap = 4;
    while (new_cap < new_len) new_cap += new_cap >> 1;
    
    int64_t *new_base = (int64_t *)malloc(16 + new_cap * 8);
    if (!new_base) return NULL;
    
    new_base[0] = new_cap;
    new_base[1] = new_len;
    int64_t *new_data = new_base + 1;
    
    memcpy(new_data + 1, (int64_t *)list_ptr + 1 + start, (size_t)new_len * 8);
    
    return new_data;
}

// mire_strings_split_list - splits string and returns a list (for strings.split)
void *mire_strings_split_list(const char *input, const char *delimiter) {
    if (input == NULL || delimiter == NULL) {
        return mire_list_create(0, sizeof(void *));
    }
    
    size_t delim_len = strlen(delimiter);
    size_t input_len = strlen(input);
    if (delim_len == 0) {
        void *list = mire_list_create(1, sizeof(void *));
        if (list == NULL) {
            return NULL;
        }
        char *copy = mire_strdup_raw(input);
        if (copy != NULL) {
            mire_list_push_ptr(list, copy);
        }
        return list;
    }
    
    size_t count = 1;
    const char *cursor = input;
    const char *match = NULL;
    while ((match = strstr(cursor, delimiter)) != NULL) {
        count++;
        cursor = match + delim_len;
    }
    
    void *list = mire_list_create(count, sizeof(void *));
    if (list == NULL) {
        return NULL;
    }

    const char *segment_start = input;
    cursor = input;
    while ((match = strstr(cursor, delimiter)) != NULL) {
        size_t segment_len = (size_t)(match - segment_start);
        char *copy = mire_strdup_raw_n(segment_start, segment_len);
        if (copy != NULL) {
            mire_list_push_ptr(list, copy);
        }
        segment_start = match + delim_len;
        cursor = segment_start;
    }

    size_t tail_len = input_len - (size_t)(segment_start - input);
    char *tail = mire_strdup_raw_n(segment_start, tail_len);
    if (tail != NULL) {
        mire_list_push_ptr(list, tail);
    }
    return list;
}

// Old version kept for backward compatibility - returns concatenated string
char *mire_strings_split(const char *input, const char *delimiter) {
    if (input == NULL || delimiter == NULL) {
        return mire_managed_from_slice("", 0);
    }
    
    size_t delim_len = strlen(delimiter);
    if (delim_len == 0) {
        return mire_managed_from_slice(input, strlen(input));
    }
    
    size_t input_len = strlen(input);
    
    size_t count = 1;
    const char *p = input;
    while ((p = strstr(p, delimiter)) != NULL) {
        count++;
        p += delim_len;
    }
    
    char **parts = (char **)malloc(count * sizeof(char *));
    if (parts == NULL) {
        return mire_managed_from_slice("", 0);
    }
    
    char *input_copy = mire_strdup_raw(input);
    char *token = strtok(input_copy, delimiter);
    size_t idx = 0;
    
    while (token != NULL && idx < count) {
        parts[idx++] = mire_strdup_raw(token);
        token = strtok(NULL, delimiter);
    }
    
    free(input_copy);
    
    size_t total_len = 0;
    for (size_t i = 0; i < idx; i++) {
        total_len += strlen(parts[i]) + 1;
    }
    
    char *result = mire_managed_alloc(total_len);
    if (result == NULL) {
        for (size_t i = 0; i < idx; i++) {
            free(parts[i]);
        }
        free(parts);
        return mire_managed_from_slice("", 0);
    }
    
    result[0] = '\0';
    for (size_t i = 0; i < idx; i++) {
        if (i > 0) {
            strcat(result, " ");
        }
        strcat(result, parts[i]);
    }
    
    for (size_t i = 0; i < idx; i++) {
        free(parts[i]);
    }
    free(parts);
    
    return result;
}

char *mire_strings_replace_first(const char *input, const char *from, const char *to) {
    if (input == NULL || from == NULL || to == NULL) {
        return mire_managed_from_slice("", 0);
    }

    size_t input_len = strlen(input);
    size_t from_len = strlen(from);
    size_t to_len = strlen(to);
    if (from_len == 0) {
        return mire_managed_from_slice(input, input_len);
    }

    const char *match = strstr(input, from);
    if (match == NULL) {
        return mire_managed_from_slice(input, input_len);
    }

    size_t out_len = input_len - from_len + to_len;
    char *out = mire_managed_alloc(out_len);
    if (out == NULL) {
        return mire_managed_from_slice(input, input_len);
    }

    size_t prefix_len = (size_t)(match - input);
    if (prefix_len > 0) {
        memcpy(out, input, prefix_len);
    }
    if (to_len > 0) {
        memcpy(out + prefix_len, to, to_len);
    }
    const char *suffix_start = match + from_len;
    size_t suffix_len = input_len - (size_t)(suffix_start - input);
    if (suffix_len > 0) {
        memcpy(out + prefix_len + to_len, suffix_start, suffix_len);
    }
    out[out_len] = '\0';
    return out;
}

int64_t mire_strings_starts_with(const char *str, const char *prefix) {
    if (str == NULL || prefix == NULL) {
        return 0;
    }
    size_t prefix_len = strlen(prefix);
    return strncmp(str, prefix, prefix_len) == 0 ? 1 : 0;
}

int64_t mire_strings_ends_with(const char *str, const char *suffix) {
    if (str == NULL || suffix == NULL) {
        return 0;
    }
    size_t str_len = strlen(str);
    size_t suffix_len = strlen(suffix);
    if (suffix_len > str_len) {
        return 0;
    }
    return strcmp(str + str_len - suffix_len, suffix) == 0 ? 1 : 0;
}

char *mire_strings_join(char **parts, size_t count, const char *delimiter) {
    if (parts == NULL || count == 0) {
        return mire_managed_from_slice("", 0);
    }
    
    if (delimiter == NULL) {
        delimiter = "";
    }
    size_t delim_len = strlen(delimiter);
    
    size_t total_len = 0;
    for (size_t i = 0; i < count; i++) {
        if (parts[i] != NULL) {
            total_len += strlen(parts[i]);
        }
    }
    
    if (count > 1 && delim_len > 0) {
        total_len += (count - 1) * delim_len;
    }
    
    char *result = mire_managed_alloc(total_len);
    if (result == NULL) {
        return mire_managed_from_slice("", 0);
    }
    
    result[0] = '\0';
    for (size_t i = 0; i < count; i++) {
        if (i > 0 && delim_len > 0) {
            strcat(result, delimiter);
        }
        if (parts[i] != NULL) {
            strcat(result, parts[i]);
        }
    }
    
    return result;
}

char *mire_strings_trim(const char *input) {
    if (input == NULL) {
        return mire_managed_from_slice("", 0);
    }
    
    const char *start = input;
    const char *end = input + strlen(input);
    
    while (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r') {
        start++;
    }
    
    while (end > start && (*(end - 1) == ' ' || *(end - 1) == '\t' || *(end - 1) == '\n' || *(end - 1) == '\r')) {
        end--;
    }
    
    size_t len = end - start;
    char *result = mire_managed_alloc(len);
    if (result == NULL) {
        return mire_managed_from_slice("", 0);
    }
    
    memcpy(result, start, len);
    result[len] = '\0';
    
    return result;
}

void *mire_dict_keys(void *dict_ptr) {
    MireDict *dict = (MireDict *)dict_ptr;
    if (dict == NULL || dict->len == 0) {
        return NULL;
    }
    
    int64_t new_cap = 4;
    while (new_cap < dict->len) new_cap += new_cap >> 1;
    
    int64_t *result = (int64_t *)malloc(16 + new_cap * 8);
    if (result == NULL) return NULL;
    
    result[0] = new_cap;
    result[1] = dict->len;
    int64_t *list_ptr = result + 1;
    int64_t *data = list_ptr + 1;
    
    for (int64_t i = 0; i < dict->len; i++) {
        if (dict->key_kind == MIRE_KIND_SCALAR) {
            data[i] = dict->entries[i].key_i64;
        } else {
            data[i] = (int64_t)dict->entries[i].key_str;
        }
    }
    
    return list_ptr;
}

void *mire_dict_values(void *dict_ptr) {
    MireDict *dict = (MireDict *)dict_ptr;
    if (dict == NULL || dict->len == 0) {
        return NULL;
    }
    
    int64_t new_cap = 4;
    while (new_cap < dict->len) new_cap += new_cap >> 1;
    
    int64_t *result = (int64_t *)malloc(16 + new_cap * 8);
    if (result == NULL) return NULL;
    
    result[0] = new_cap;
    result[1] = dict->len;
    int64_t *list_ptr = result + 1;
    int64_t *data = list_ptr + 1;
    
    for (int64_t i = 0; i < dict->len; i++) {
        if (dict->value_kind == MIRE_KIND_PTR) {
            data[i] = (int64_t)(dict->value_storage + i * dict->value_size);
        } else {
            data[i] = *(int64_t *)(dict->value_storage + i * dict->value_size);
        }
    }
    
    return list_ptr;
}

// Get command line arguments as a list of strings
// Returns a list of Mire-managed strings (each arg as proper MireManagedString)
void *mire_get_args(int argc, char **argv) {
    void *list_ptr = mire_list_create(argc, sizeof(void *));
    if (!list_ptr) return NULL;
    
    for (int i = 0; i < argc; i++) {
        char *arg = argv[i];
        size_t len = strlen(arg);
        
        // Allocate MireManagedString format: [len, cap, data...]
        size_t total_size = sizeof(size_t) * 2 + len + 1;
        MireManagedString *str = (MireManagedString *)malloc(total_size);
        if (!str) continue;
        
        str->len = len;
        str->cap = len + 1;
        memcpy(str->data, arg, len + 1);
        
        // Register for garbage collection
        mire_managed_register(str->data);
        
        // Push string pointer (not the string data) to list
        list_ptr = mire_list_push_ptr(list_ptr, str->data);
    }
    
    return list_ptr;
}

// ==================== File System Functions ====================

int mire_fs_write(const char *path, const char *content) {
    FILE *f = fopen(path, "w");
    if (!f) return 0;
    fputs(content, f);
    fclose(f);
    return 1;
}

int mire_fs_append(const char *path, const char *content) {
    FILE *f = fopen(path, "a");
    if (!f) return 0;
    fputs(content, f);
    fclose(f);
    return 1;
}

char *mire_fs_read(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) return mire_strdup_raw("");
    
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char *buf = (char *)malloc(size + 1);
    fread(buf, 1, size, f);
    buf[size] = '\0';
    fclose(f);
    
    char *result = mire_strdup_raw(buf);
    free(buf);
    return result;
}

int mire_fs_copy(const char *src, const char *dst) {
    FILE *in = fopen(src, "rb");
    if (!in) return 0;
    
    FILE *out = fopen(dst, "wb");
    if (!out) { fclose(in); return 0; }
    
    char buf[4096];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), in)) > 0) {
        fwrite(buf, 1, n, out);
    }
    
    fclose(in);
    fclose(out);
    return 1;
}

int mire_fs_move(const char *src, const char *dst) {
    if (mire_fs_copy(src, dst)) {
        remove(src);
        return 1;
    }
    return 0;
}

int mire_fs_drop(const char *path) {
    return remove(path) == 0 ? 1 : 0;
}

int mire_fs_mkdir(const char *path) {
    return mkdir(path, 0755) == 0 ? 1 : 0;
}

int mire_fs_rmdir(const char *path) {
    return rmdir(path) == 0 ? 1 : 0;
}

int64_t mire_fs_exists(const char *path) {
    return access(path, F_OK) == 0 ? 1 : 0;
}

int64_t mire_fs_is_dir(const char *path) {
    struct stat st;
    if (stat(path, &st) == 0) {
        return S_ISDIR(st.st_mode) ? 1 : 0;
    }
    return 0;
}

int64_t mire_fs_size(const char *path) {
    struct stat st;
    if (stat(path, &st) == 0) {
        return st.st_size;
    }
    return 0;
}

void *mire_fs_list(const char *path) {
    void *list_ptr = mire_list_create(16, sizeof(void *));
    if (!list_ptr) return NULL;
    
    DIR *d = opendir(path);
    if (!d) return list_ptr;
    
    struct dirent *entry;
    while ((entry = readdir(d)) != NULL) {
        if (entry->d_name[0] == '.') continue;
        
        char *name = mire_strdup_raw(entry->d_name);
        list_ptr = mire_list_push_ptr(list_ptr, name);
    }
    
    closedir(d);
    return list_ptr;
}

char *mire_fs_join(const char *a, const char *b) {
    size_t len = strlen(a) + strlen(b) + 2;
    char *result = (char *)malloc(len);
    snprintf(result, len, "%s/%s", a, b);
    char *managed = mire_strdup_raw(result);
    free(result);
    return managed;
}

char *mire_fs_dir(const char *path) {
    char *copy = mire_strdup_raw(path);
    char *last = strrchr(copy, '/');
    if (last) {
        *last = '\0';
        return copy;
    }
    free(copy);
    return mire_strdup_raw(".");
}

char *mire_fs_name(const char *path) {
    const char *last = strrchr(path, '/');
    if (last) {
        return mire_strdup_raw(last + 1);
    }
    return mire_strdup_raw(path);
}

char *mire_fs_ext(const char *path) {
    const char *last = strrchr(path, '.');
    if (last && last != path) {
        return mire_strdup_raw(last + 1);
    }
    return mire_strdup_raw("");
}

// ==================== Process Functions ====================

char *mire_proc_run(const char *cmd) {
    FILE *p = popen(cmd, "r");
    if (!p) return mire_strdup_raw("");
    
    char buf[4096];
    size_t n = fread(buf, 1, sizeof(buf) - 1, p);
    buf[n] = '\0';
    pclose(p);
    
    return mire_strdup_raw(buf);
}

char *mire_proc_exec(const char *cmd) {
    return mire_proc_run(cmd);
}

int64_t mire_proc_wait(int pid) {
    int status;
    waitpid(pid, &status, 0);
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

int mire_proc_kill(int pid) {
    return kill(pid, SIGTERM) == 0 ? 1 : 0;
}

void mire_proc_exit(int code) {
    exit(code);
}

char *mire_proc_shell(const char *cmd) {
    FILE *p = popen(cmd, "r");
    if (!p) return mire_strdup_raw("");
    
    char buf[4096];
    size_t n = fread(buf, 1, sizeof(buf) - 1, p);
    buf[n] = '\0';
    pclose(p);
    
    return mire_strdup_raw(buf);
}

int mire_proc_exists(int pid) {
    return kill(pid, 0) == 0 ? 1 : 0;
}

// ==================== Environment Functions ====================

char *mire_env_get(const char *name) {
    char *val = getenv(name);
    return val ? mire_strdup_raw(val) : mire_strdup_raw("");
}

int mire_env_set(const char *name, const char *value) {
    return setenv(name, value, 1) == 0 ? 1 : 0;
}

char *mire_env_cwd(void) {
    char buf[4096];
    if (getcwd(buf, sizeof(buf))) {
        return mire_strdup_raw(buf);
    }
    return mire_strdup_raw("");
}

void *mire_env_all(void) {
    void *list_ptr = mire_list_create(32, sizeof(void *));
    if (!list_ptr) return NULL;
    
    extern char **environ;
    for (int i = 0; environ[i] != NULL; i++) {
        char *env_str = mire_strdup_raw(environ[i]);
        list_ptr = mire_list_push_ptr(list_ptr, env_str);
    }
    
    return list_ptr;
}

// ==================== I/O: mire_read_line ====================
// Reads a line from stdin, optionally with a prompt.
// If prompt is non-NULL and non-empty, prints it to stdout (no newline) and flushes.
// Returns a heap-allocated, managed string (registered for GC) without the trailing newline.
// On EOF/error, returns an empty managed string.
// NOTE: stdin I/O builtins are planned for migration to kioto ABI
//       (see __kioto_io_readln in kioto_abi_v1.md §3.11).
char *mire_read_line(const char *prompt) {
    if (prompt != NULL && prompt[0] != '\0') {
        printf("%s", prompt);
        fflush(stdout);
    }

    size_t cap = 128;
    size_t len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) return mire_managed_from_slice("", 0);

    int c;
    while ((c = getchar()) != EOF && c != '\n') {
        if (len + 1 >= cap) {
            cap += cap >> 1;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) { free(buf); return mire_managed_from_slice("", 0); }
            buf = nb;
        }
        buf[len++] = (char)c;
    }
    buf[len] = '\0';

    char *result = mire_managed_from_slice(buf, len);
    free(buf);
    return result;
}

// ==================== Kioto ABI v1 — Adapter Layer ====================
// These functions implement the __kioto_* contract defined in
// kioto_abi_v1.md so that Kioto modules can call them via extern fn
// instead of relying on Avenys builtins.

// ── fs ────────────────────────────────────────────────────────────────
char *__kioto_fs_read(const char *path) { return mire_fs_read(path); }
int __kioto_fs_write(const char *path, const char *data) { return mire_fs_write(path, data); }
int __kioto_fs_append(const char *path, const char *data) { return mire_fs_append(path, data); }
int64_t __kioto_fs_exists(const char *path) { return mire_fs_exists(path); }
int64_t __kioto_fs_is_dir(const char *path) { return mire_fs_is_dir(path); }
int64_t __kioto_fs_size(const char *path) { return mire_fs_size(path); }
int __kioto_fs_copy(const char *src, const char *dst) { return mire_fs_copy(src, dst); }
int __kioto_fs_move(const char *src, const char *dst) { return mire_fs_move(src, dst); }
int __kioto_fs_delete(const char *path) { return mire_fs_drop(path); }
void *__kioto_fs_list(const char *path) { return mire_fs_list(path); }
int __kioto_fs_mkdir(const char *path) { return mire_fs_mkdir(path); }
int __kioto_fs_rmdir(const char *path) { return mire_fs_rmdir(path); }
char *__kioto_fs_join(const char *a, const char *b) { return mire_fs_join(a, b); }
char *__kioto_fs_dirname(const char *path) { return mire_fs_dir(path); }
char *__kioto_fs_basename(const char *path) { return mire_fs_name(path); }
char *__kioto_fs_extension(const char *path) { return mire_fs_ext(path); }

// ── env ───────────────────────────────────────────────────────────────
char *__kioto_env_get(const char *key) { return mire_env_get(key); }
int __kioto_env_set(const char *key, const char *value) { return mire_env_set(key, value); }
void *__kioto_env_all(void) { return mire_env_all(); }
void *__kioto_env_args(void) { extern void *mire_get_args(int, char **); return mire_get_args(0, NULL); }
char *__kioto_env_cwd(void) { return mire_env_cwd(); }

// ── strings ───────────────────────────────────────────────────────────
char *__kioto_strings_to_upper(const char *s) { return mire_string_to_upper(s); }
char *__kioto_strings_to_lower(const char *s) { return mire_string_to_lower(s); }
char *__kioto_strings_trim(const char *s) { return mire_strings_trim(s); }
char *__kioto_strings_strip(const char *s) { return mire_strings_trim(s); }
char *__kioto_strings_replace(const char *s, const char *old, const char *rep) { return mire_strings_replace(s, old, rep); }
char *__kioto_strings_replace_first(const char *s, const char *old, const char *rep) { return mire_strings_replace_first(s, old, rep); }
int64_t __kioto_strings_contains(const char *s, const char *sub) { return mire_strings_contains(s, sub); }
int64_t __kioto_strings_starts_with(const char *s, const char *prefix) { return mire_strings_starts_with(s, prefix); }
int64_t __kioto_strings_ends_with(const char *s, const char *suffix) { return mire_strings_ends_with(s, suffix); }
int64_t __kioto_strings_len(const char *s) { return (int64_t)strlen(s); }
void *__kioto_strings_split(const char *s, const char *sep) { return mire_strings_split_list(s, sep); }
char *__kioto_strings_join(void *parts, const char *sep) { extern int64_t mire_list_len(void *); return mire_strings_join((char **)parts, (size_t)mire_list_len(parts), sep); }
char *__kioto_strings_substr(const char *s, int64_t start, int64_t len) { return mire_strings_substr(s, start, len); }
char *__kioto_strings_pad_left(const char *s, int64_t w, const char *pad) { return mire_strings_pad_left(s, w, pad); }
char *__kioto_strings_pad_right(const char *s, int64_t w, const char *pad) { return mire_strings_pad_right(s, w, pad); }
char *__kioto_strings_repeat(const char *s, int64_t times) { return mire_strings_repeat(s, times); }

// ── math (no libm dependency — inline impls) ──────────────────────────
static int64_t mire_double_round(double x) {
    if (x < 0.0) return (int64_t)(x - 0.5);
    return (int64_t)(x + 0.5);
}
static int64_t mire_double_floor(double x) {
    int64_t i = (int64_t)x;
    return (double)i > x ? i - 1 : i;
}
static int64_t mire_double_ceil(double x) {
    int64_t i = (int64_t)x;
    return (double)i < x ? i + 1 : i;
}
int64_t __kioto_math_round(double x) { return mire_double_round(x); }
int64_t __kioto_math_floor(double x) { return mire_double_floor(x); }
int64_t __kioto_math_ceil(double x) { return mire_double_ceil(x); }

// ── time ──────────────────────────────────────────────────────────────
int64_t __kioto_time_mark(void) { return mire_wall_mark_ns(); }
int64_t __kioto_time_elapsed_ms(int64_t start) { return mire_wall_elapsed_ms(start); }
int64_t __kioto_time_elapsed_ns(int64_t start) { extern int64_t mire_clock_ns(clockid_t); return mire_clock_ns(CLOCK_MONOTONIC) - start; }

// ── cpu ───────────────────────────────────────────────────────────────
int64_t __kioto_cpu_mark(void) { return mire_cpu_mark_ns(); }
int64_t __kioto_cpu_elapsed_ms(int64_t start) { return mire_cpu_elapsed_ms(start); }
int64_t __kioto_cpu_elapsed_ns(int64_t start) { extern int64_t mire_clock_ns(clockid_t); return mire_clock_ns(CLOCK_PROCESS_CPUTIME_ID) - start; }
int64_t __kioto_cpu_cycles_est(int64_t start) { return mire_cpu_cycles_est(start); }

// ── proc ──────────────────────────────────────────────────────────────
char *__kioto_proc_run(const char *cmd) { return mire_proc_run(cmd); }
char *__kioto_proc_exec(const char *cmd) { return mire_proc_exec(cmd); }
char *__kioto_proc_shell(const char *cmd) { return mire_proc_shell(cmd); }
int64_t __kioto_proc_wait(int64_t pid) { return mire_proc_wait((int)pid); }
int __kioto_proc_kill(int64_t pid) { return mire_proc_kill((int)pid); }
void __kioto_proc_exit(int64_t status) { mire_proc_exit((int)status); }
int64_t __kioto_proc_exists(int64_t pid) { return mire_proc_exists((int)pid); }

// ── lists ─────────────────────────────────────────────────────────────
int64_t __kioto_lists_len(void *list) { return mire_list_len(list); }
int64_t __kioto_lists_get_i64(void *list, int64_t index) {
    if (!list) return 0;
    int64_t len = mire_list_len(list);
    if (index < 0 || index >= len) return 0;
    return ((int64_t *)list)[index + 1];
}
void *__kioto_lists_get_ptr(void *list, int64_t index) {
    if (!list) return NULL;
    int64_t len = mire_list_len(list);
    if (index < 0 || index >= len) return NULL;
    return ((void **)((int64_t *)list + 1))[index];
}
void *__kioto_lists_push_i64(void *list, int64_t value) { return mire_list_push_i64(list, value); }
void *__kioto_lists_push_ptr(void *list, void *value) { return mire_list_push_ptr(list, value); }
int64_t __kioto_lists_pop(void *list) { return mire_list_pop_i64(list); }
void *__kioto_lists_slice(void *list, int64_t start, int64_t end) { return mire_list_slice(list, start, end); }
void *__kioto_lists_concat(void *a, void *b) { return mire_list_concat(a, b); }
void *__kioto_lists_remove(void *list, int64_t index) {
    if (!list) return list;
    int64_t len = mire_list_len(list);
    if (index < 0 || index >= len) return list;
    int64_t *data = (int64_t *)list;
    for (int64_t i = index; i < len - 1; i++) {
        data[i + 1] = data[i + 2];
    }
    data[0] = len - 1;
    return list;
}
void *__kioto_lists_clear(void *list) {
    if (!list) return list;
    ((int64_t *)list)[0] = 0;
    return list;
}

// ── dicts ─────────────────────────────────────────────────────────────
int64_t __kioto_dicts_len(void *dict) {
    if (!dict) return 0;
    return ((MireDict *)dict)->len;
}
void *__kioto_dicts_get(void *dict, const char *key) {
    return mire_dict_get_ptr(dict, MIRE_KIND_STR, 0, (void *)key, NULL);
}
void *__kioto_dicts_set(void *dict, const char *key, void *value) {
    return mire_dict_set_ptr(dict, MIRE_KIND_STR, MIRE_KIND_STR, 0, (void *)key, value);
}
void *__kioto_dicts_set_i64(void *dict, const char *key, int64_t value) {
    return mire_dict_set_i64(dict, MIRE_KIND_STR, MIRE_KIND_SCALAR, 0, (void *)key, value);
}
int64_t __kioto_dicts_has(void *dict, const char *key) {
    if (!dict) return 0;
    uint64_t hash = mire_hash_string(key);
    return mire_dict_find((MireDict *)dict, 0, (void *)key, hash) >= 0 ? 1 : 0;
}
void *__kioto_dicts_remove(void *dict, const char *key) {
    if (!dict) return dict;
    MireDict *d = (MireDict *)dict;
    uint64_t hash = mire_hash_string(key);
    int64_t idx = mire_dict_find(d, 0, (void *)key, hash);
    if (idx >= 0) {
        d->entries[idx].hash = 0;
        d->entries[idx].next = -2;
    }
    return dict;
}
void *__kioto_dicts_keys(void *dict) { return mire_dict_keys(dict); }
void *__kioto_dicts_values(void *dict) { return mire_dict_values(dict); }

// ── mem ───────────────────────────────────────────────────────────────
int64_t __kioto_mem_process_bytes(void) { return mire_mem_process_bytes(); }
char *__kioto_mem_format(int64_t bytes) { return mire_mem_format(bytes); }
