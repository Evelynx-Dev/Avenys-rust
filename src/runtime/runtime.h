#ifndef MIRE_RUNTIME_H
#define MIRE_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

// ── Managed strings ──────────────────────────────────────────────────
// Format: header (len, cap) + data[cap+1] (inline, no pointer indirection)
typedef struct {
    size_t len;
    size_t cap;
    char data[];
} MireManagedString;

char *rt_managed_alloc(size_t len);
char *rt_managed_from_slice(const char *src, size_t len);
char *rt_managed_from_cstr(const char *src);
char *rt_managed_printf_i64(const char *fmt, long long value);
char *rt_managed_printf_f64(const char *fmt, double value);
void  rt_managed_free(char *value);
size_t rt_managed_len(const char *value);
int   rt_managed_contains(const char *data_ptr);
void  rt_managed_register(char *data_ptr);
void  rt_managed_unregister(char *data_ptr);

// low-level helpers
char *rt_strdup_raw(const char *src);
char *rt_strdup_raw_n(const char *src, size_t len);
char *rt_alloc_printf_raw_i64(const char *fmt, long long value);
size_t rt_string_growth_cap(size_t min_cap);

// ── String operations ────────────────────────────────────────────────
char *rt_string_copy(const char *value);
char *rt_string_concat(const char *left, const char *right);
char *rt_string_append_owned(char *value, const char *suffix);

char *rt_i64_to_string(int64_t value);
char *rt_bool_to_string(int64_t value);
char *rt_f64_to_string(double value);

char *rt_string_to_upper(const char *value);
char *rt_string_to_lower(const char *value);
char  rt_unicode_to_lower(unsigned char c);
char  rt_unicode_to_upper(unsigned char c);

// ── List operations (inline-storage dynamic arrays) ──────────────────
// Format: [capacity, length, data...]
void *rt_list_create(int64_t initial_cap, int64_t elem_size);
int64_t rt_list_len(void *list_ptr);
void *rt_list_push_i64(void *list_ptr, int64_t value);
void *rt_list_push_ptr(void *list_ptr, void *value);
void *rt_list_push_scalar(void *list_ptr, int64_t value, int64_t elem_size);
int64_t rt_list_pop_i64(void *list_ptr);
void *rt_list_concat(void *left_ptr, void *right_ptr);
void *rt_list_slice(void *list_ptr, int64_t start, int64_t end);
void *rt_list_remove(void *list_ptr, int64_t index);
void *rt_list_clear(void *list_ptr);
int64_t rt_list_get_i64(void *list_ptr, int64_t index);
void *rt_list_get_ptr(void *list_ptr, int64_t index);

// ── Dict operations (hash table, open addressing) ────────────────────
// Key/value kinds
enum {
    MIRE_KIND_SCALAR = 1,
    MIRE_KIND_BOOL = 2,
    MIRE_KIND_STR = 3,
    MIRE_KIND_MAP = 4,
    MIRE_KIND_PTR = 5,
};

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

int64_t rt_dict_len(void *dict_ptr);
void *rt_dict_ensure(void *dict_ptr);
void *rt_dict_ensure_kind(void *dict_ptr, int64_t key_kind, int64_t value_kind);

int64_t rt_dict_get_i64(void *dict_ptr, int64_t key_kind, int64_t key_i64,
                         void *key_ptr, int64_t default_value);
void  *rt_dict_set_i64(void *dict_ptr, int64_t key_kind, int64_t value_kind,
                         int64_t key_i64, void *key_ptr, int64_t value);
void  *rt_dict_get_ptr(void *dict_ptr, int64_t key_kind, int64_t key_i64,
                         void *key_ptr, void *default_value);
void  *rt_dict_set_ptr(void *dict_ptr, int64_t key_kind, int64_t value_kind,
                         int64_t key_i64, void *key_ptr, void *value);
int64_t rt_dict_has(void *dict_ptr, int64_t key_kind, int64_t key_i64, void *key_ptr);
void  *rt_dict_remove(void *dict_ptr, int64_t key_kind, int64_t key_i64, void *key_ptr);
char  *rt_dict_to_string(void *dict_ptr);
void  *rt_dict_keys(void *dict_ptr);
void  *rt_dict_values(void *dict_ptr);

// ── Runtime utilities ────────────────────────────────────────────────
void rt_panic(const char *message);

#endif // MIRE_RUNTIME_H
