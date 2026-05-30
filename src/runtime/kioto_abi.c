// kioto_abi.c — Kioto ABI v1 compatibility shim
// Delegates all __kioto_* and mire_* entry points to the new Runtime Core + PAL.
// This file will be removed once the LLVM codegen is updated to call rt_*/pal_* directly.

#include "runtime.h"
#include "../pal/pal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
#include <time.h>
#include <math.h>

// ══════════════════════════════════════════════════════════════════════
// mire_* compatibility (called from LLVM codegen)
// ══════════════════════════════════════════════════════════════════════

void mire_runtime_panic(const char *message) { rt_panic(message); }

// Managed strings
void mire_string_free(char *value) { rt_managed_free(value); }
char *mire_managed_from_slice(const char *src, size_t len) { return rt_managed_from_slice(src, len); }
char *mire_managed_alloc(size_t len) { return rt_managed_alloc(len); }
char *mire_managed_printf_i64(const char *fmt, long long value) { return rt_managed_printf_i64(fmt, value); }
char *mire_managed_printf_f64(const char *fmt, double value) { return rt_managed_printf_f64(fmt, value); }
char *mire_strdup_raw(const char *src) { return rt_strdup_raw(src); }
int mire_managed_contains(const char *data_ptr) { return rt_managed_contains(data_ptr); }

// String conversions
char *mire_i64_to_string(int64_t value) { return rt_i64_to_string(value); }
char *mire_bool_to_string(int64_t value) { return rt_bool_to_string(value); }
char *mire_f64_to_string(double value) { return rt_f64_to_string(value); }

// String operations
char *mire_string_copy(const char *value) { return rt_string_copy(value); }
char *mire_string_concat(const char *left, const char *right) { return rt_string_concat(left, right); }
char *mire_string_append_owned(char *value, const char *suffix) { return rt_string_append_owned(value, suffix); }
char mire_unicode_to_lower(unsigned char c) { return rt_unicode_to_lower(c); }
char mire_unicode_to_upper(unsigned char c) { return rt_unicode_to_upper(c); }
char *mire_string_to_upper(const char *value) { return rt_string_to_upper(value); }
char *mire_string_to_lower(const char *value) { return rt_string_to_lower(value); }
char *mire_strings_replace(const char *input, const char *from, const char *to) {
    if (!input) input = "";
    if (!from || !*from) return rt_managed_from_slice(input, strlen(input));
    if (!to) to = "";
    const char *pos = strstr(input, from);
    if (!pos) return rt_managed_from_slice(input, strlen(input));
    size_t prefix_len = (size_t)(pos - input);
    size_t from_len = strlen(from);
    size_t to_len = strlen(to);
    size_t suffix_len = strlen(pos + from_len);
    char *out = rt_managed_alloc(prefix_len + to_len + suffix_len);
    if (!out) return rt_managed_from_slice("", 0);
    memcpy(out, input, prefix_len);
    memcpy(out + prefix_len, to, to_len);
    memcpy(out + prefix_len + to_len, pos + from_len, suffix_len);
    out[prefix_len + to_len + suffix_len] = '\0';
    return out;
}
int64_t mire_strings_contains(const char *input, const char *needle) {
    if (!input || !needle) return 0;
    return strstr(input, needle) != NULL ? 1 : 0;
}
char *mire_strings_substr(const char *input, int64_t start, int64_t length) {
    if (!input) return rt_managed_from_slice("", 0);
    size_t len = strlen(input);
    if (start < 0) start = 0;
    if ((size_t)start >= len || length <= 0) return rt_managed_from_slice("", 0);
    if ((size_t)(start + length) > len) length = (int64_t)(len - (size_t)start);
    return rt_managed_from_slice(input + start, (size_t)length);
}
static char *pad_core(const char *input, int64_t width, const char *pad, int left_pad) {
    if (!input) input = "";
    if (!pad) pad = " ";
    size_t ilen = strlen(input);
    size_t plen = strlen(pad);
    if (plen == 0 || (int64_t)ilen >= width) return rt_managed_from_slice(input, ilen);
    size_t pad_total = (size_t)(width - (int64_t)ilen);
    size_t repeats = pad_total / plen + 1;
    size_t buf_len = repeats * plen;
    char *pad_buf = (char *)malloc(buf_len);
    if (!pad_buf) return rt_managed_from_slice(input, ilen);
    for (size_t i = 0; i < repeats; i++) memcpy(pad_buf + i * plen, pad, plen);
    size_t result_len = (size_t)width;
    char *out = rt_managed_alloc(result_len);
    if (!out) { free(pad_buf); return rt_managed_from_slice("", 0); }
    if (left_pad) {
        memcpy(out, pad_buf, pad_total);
        memcpy(out + pad_total, input, ilen);
    } else {
        memcpy(out, input, ilen);
        memcpy(out + ilen, pad_buf, pad_total);
    }
    out[result_len] = '\0';
    free(pad_buf);
    return out;
}
char *mire_strings_pad_left(const char *input, int64_t width, const char *pad) {
    return pad_core(input, width, pad, 1);
}
char *mire_strings_pad_right(const char *input, int64_t width, const char *pad) {
    return pad_core(input, width, pad, 0);
}
char *mire_strings_trim(const char *input) {
    if (!input) return rt_managed_from_slice("", 0);
    size_t len = strlen(input);
    const char *start = input;
    const char *end = input + len - 1;
    while (start <= end && (unsigned char)*start <= ' ') start++;
    while (end >= start && (unsigned char)*end <= ' ') end--;
    if (start > end) return rt_managed_from_slice("", 0);
    return rt_managed_from_slice(start, (size_t)(end - start + 1));
}
char *mire_strings_split(const char *input, const char *delimiter) {
    // Simple split with strtok
    if (!input) input = "";
    if (!delimiter || !*delimiter) delimiter = " ";
    char *copy = rt_strdup_raw(input);
    if (!copy) return rt_managed_from_slice("", 0);
    size_t cap = 16, count = 0;
    char **tokens = (char **)malloc(cap * sizeof(char *));
    if (!tokens) { free(copy); return rt_managed_from_slice("", 0); }
    char *token = strtok(copy, delimiter);
    while (token) {
        if (count >= cap) {
            cap *= 2;
            char **nt = (char **)realloc(tokens, cap * sizeof(char *));
            if (!nt) break;
            tokens = nt;
        }
        tokens[count++] = token;
        token = strtok(NULL, delimiter);
    }
    size_t total_len = 0;
    for (size_t i = 0; i < count; i++) {
        total_len += strlen(tokens[i]) + 2;
    }
    if (count > 0) total_len += 2;
    char *result = rt_managed_alloc(total_len);
    if (result) {
        size_t pos = 0;
        result[pos++] = '[';
        for (size_t i = 0; i < count; i++) {
            if (i > 0) { result[pos++] = ','; result[pos++] = ' '; }
            result[pos++] = '"';
            size_t tlen = strlen(tokens[i]);
            memcpy(result + pos, tokens[i], tlen);
            pos += tlen;
            result[pos++] = '"';
        }
        result[pos++] = ']';
        result[pos] = '\0';
    }
    free(tokens);
    free(copy);
    return result ? result : rt_managed_from_slice("[]", 2);
}
char *mire_strings_replace_first(const char *input, const char *from, const char *to) {
    if (!input || !from || !*from) return rt_managed_from_slice(input ? input : "", input ? strlen(input) : 0);
    if (!to) to = "";
    const char *pos = strstr(input, from);
    if (!pos) return rt_managed_from_slice(input, strlen(input));
    size_t prefix = (size_t)(pos - input);
    size_t from_len = strlen(from);
    size_t to_len = strlen(to);
    size_t suffix = strlen(pos + from_len);
    char *out = rt_managed_alloc(prefix + to_len + suffix);
    if (!out) return rt_managed_from_slice("", 0);
    memcpy(out, input, prefix);
    memcpy(out + prefix, to, to_len);
    memcpy(out + prefix + to_len, pos + from_len, suffix);
    out[prefix + to_len + suffix] = '\0';
    return out;
}
int64_t mire_strings_starts_with(const char *str, const char *prefix) {
    if (!str || !prefix) return 0;
    size_t slen = strlen(str), plen = strlen(prefix);
    if (plen > slen) return 0;
    return strncmp(str, prefix, plen) == 0 ? 1 : 0;
}
int64_t mire_strings_ends_with(const char *str, const char *suffix) {
    if (!str || !suffix) return 0;
    size_t slen = strlen(str), suflen = strlen(suffix);
    if (suflen > slen) return 0;
    return strcmp(str + slen - suflen, suffix) == 0 ? 1 : 0;
}
char *mire_strings_join(char **parts, size_t count, const char *delimiter) {
    if (!parts || count == 0) return rt_managed_from_slice("", 0);
    if (!delimiter) delimiter = "";
    size_t dlen = strlen(delimiter);
    size_t total = 0;
    for (size_t i = 0; i < count; i++) {
        if (parts[i]) total += strlen(parts[i]);
    }
    total += dlen * (count - 1);
    char *out = rt_managed_alloc(total);
    if (!out) return rt_managed_from_slice("", 0);
    size_t pos = 0;
    for (size_t i = 0; i < count; i++) {
        if (i > 0 && dlen > 0) { memcpy(out + pos, delimiter, dlen); pos += dlen; }
        if (parts[i]) {
            size_t plen = strlen(parts[i]);
            memcpy(out + pos, parts[i], plen);
            pos += plen;
        }
    }
    out[pos] = '\0';
    return out;
}

// Lists
void *mire_list_create(int64_t initial_cap, int64_t elem_size) { return rt_list_create(initial_cap, elem_size); }
void *mire_list_push_i64(void *list_ptr, int64_t value) { return rt_list_push_i64(list_ptr, value); }
void *mire_list_push_ptr(void *list_ptr, void *value) { return rt_list_push_ptr(list_ptr, value); }
void *mire_list_push_scalar(void *list_ptr, int64_t value, int64_t elem_size) { return rt_list_push_scalar(list_ptr, value, elem_size); }
int64_t mire_list_pop_i64(void *list_ptr) { return rt_list_pop_i64(list_ptr); }
void *mire_list_concat(void *left_ptr, void *right_ptr) { return rt_list_concat(left_ptr, right_ptr); }
void *mire_list_slice(void *list_ptr, int64_t start, int64_t end) { return rt_list_slice(list_ptr, start, end); }

// Dicts
int64_t mire_dict_get_i64(void *dict_ptr, int64_t key_kind, int64_t key_i64, void *key_ptr, int64_t default_value) {
    return rt_dict_get_i64(dict_ptr, key_kind, key_i64, key_ptr, default_value);
}
void *mire_dict_set_i64(void *dict_ptr, int64_t key_kind, int64_t value_kind, int64_t key_i64, void *key_ptr, int64_t value) {
    return rt_dict_set_i64(dict_ptr, key_kind, value_kind, key_i64, key_ptr, value);
}
void *mire_dict_get_ptr(void *dict_ptr, int64_t key_kind, int64_t key_i64, void *key_ptr, void *default_value) {
    return rt_dict_get_ptr(dict_ptr, key_kind, key_i64, key_ptr, default_value);
}
void *mire_dict_set_ptr(void *dict_ptr, int64_t key_kind, int64_t value_kind, int64_t key_i64, void *key_ptr, void *value) {
    return rt_dict_set_ptr(dict_ptr, key_kind, value_kind, key_i64, key_ptr, value);
}
char *mire_dict_to_string(void *dict_ptr) { return rt_dict_to_string(dict_ptr); }
void *mire_dict_keys(void *dict_ptr) { return rt_dict_keys(dict_ptr); }
void *mire_dict_values(void *dict_ptr) { return rt_dict_values(dict_ptr); }

// I/O
char *mire_read_line(const char *prompt) {
    if (prompt && *prompt) {
        printf("%s", prompt);
        fflush(stdout);
    }
    size_t cap = 128, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) return rt_managed_from_slice("", 0);
    int ch;
    while ((ch = getchar()) != EOF && ch != '\n') {
        if (len + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) break;
            buf = nb;
        }
        buf[len++] = (char)ch;
    }
    buf[len] = '\0';
    char *result = rt_managed_from_slice(buf, len);
    free(buf);
    return result;
}
void *mire_get_args(int argc, char **argv) {
    void *list = rt_list_create(argc > 0 ? argc : 4, 8);
    for (int i = 0; i < argc; i++) {
        char *copy = rt_strdup_raw(argv[i]);
        list = rt_list_push_ptr(list, copy);
    }
    return list;
}

// Time / CPU helpers with CLOCK_MONOTONIC fallback
static int64_t clock_ns(clockid_t clock_id) {
    struct timespec ts;
    if (clock_gettime(clock_id, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
}

// FS (delegate to PAL)
int mire_fs_write(const char *path, const char *content) { return pal_fs_write(path, content); }
int mire_fs_append(const char *path, const char *content) { return pal_fs_append(path, content); }
char *mire_fs_read(const char *path) { return pal_fs_read(path); }
int mire_fs_copy(const char *src, const char *dst) { return pal_fs_copy(src, dst); }
int mire_fs_move(const char *src, const char *dst) { return pal_fs_move(src, dst); }
int mire_fs_drop(const char *path) { return pal_fs_delete(path); }
int mire_fs_mkdir(const char *path) { return pal_fs_mkdir(path); }
int mire_fs_rmdir(const char *path) { return pal_fs_rmdir(path); }
int64_t mire_fs_exists(const char *path) { return pal_fs_exists(path); }
int64_t mire_fs_is_dir(const char *path) { return pal_fs_is_dir(path); }
int64_t mire_fs_size(const char *path) { return pal_fs_size(path); }
void *mire_fs_list(const char *path) { return pal_fs_list(path); }
char *mire_fs_join(const char *a, const char *b) { return pal_fs_join(a, b); }
char *mire_fs_dir(const char *path) { return pal_fs_dir(path); }
char *mire_fs_name(const char *path) { return pal_fs_name(path); }
char *mire_fs_ext(const char *path) { return pal_fs_ext(path); }

// Environment (delegate to PAL)
char *mire_env_get(const char *name) { return pal_env_get(name); }
int mire_env_set(const char *name, const char *value) { return pal_env_set(name, value); }
void *mire_env_all(void) { return pal_env_all(); }
char *mire_env_cwd(void) { return pal_env_cwd(); }
int mire_env_chdir(const char *path) { return pal_env_chdir(path); }

// Process (delegate to PAL)
char *mire_proc_run(const char *cmd) { return pal_proc_run(cmd); }
char *mire_proc_exec(const char *cmd) { return pal_proc_exec(cmd); }
char *mire_proc_shell(const char *cmd) { return pal_proc_shell(cmd); }
int64_t mire_proc_wait(int64_t pid) { return pal_proc_wait(pid); }
int mire_proc_kill(int64_t pid) { return pal_proc_kill(pid); }
void mire_proc_exit(int64_t status) { pal_proc_exit(status); }
int64_t mire_proc_exists(int64_t pid) { return pal_proc_exists(pid); }

// Time (delegate to PAL)
int64_t mire_wall_mark_ns(void) { return pal_time_mark(); }
int64_t mire_wall_elapsed_ms(int64_t start_ns) { return pal_time_elapsed_ms(start_ns); }
char *mire_wall_elapsed_ms_str(int64_t start_ns) {
    int64_t ms = pal_time_elapsed_ms(start_ns);
    return rt_managed_printf_i64("%lld", (long long)ms);
}
int64_t mire_wall_elapsed_str(int64_t start_ns) { return pal_time_elapsed_ms(start_ns); }

// CPU (delegate to PAL)
int64_t mire_cpu_mark_ns(void) { return pal_cpu_mark(); }
int64_t mire_cpu_elapsed_ms(int64_t start_ns) { return pal_cpu_elapsed_ms(start_ns); }
char *mire_cpu_elapsed_ms_str(int64_t start_ns) {
    int64_t ms = pal_cpu_elapsed_ms(start_ns);
    return rt_managed_printf_i64("%lld", (long long)ms);
}
int64_t mire_cpu_cycles_est(int64_t start_ns) { return pal_cpu_cycles_est(start_ns); }

// Memory (delegate to PAL)
int64_t mire_mem_process_bytes(void) { return pal_mem_process_bytes(); }
char *mire_mem_format(int64_t bytes) { return pal_mem_format(bytes); }
void *mire_mem_snapshot(void) { return pal_mem_snapshot(); }
void *mire_cpu_loadavg(void) { return pal_cpu_loadavg(); }

// GPU (delegate to PAL)
char *mire_gpu_snapshot(void) { return pal_gpu_snapshot(); }

// ══════════════════════════════════════════════════════════════════════
// __kioto_* ABI wrappers (called from std/kioto Mire modules)
// ══════════════════════════════════════════════════════════════════════

// ── FS ──
char *__kioto_fs_read(const char *path) { return pal_fs_read(path); }
int __kioto_fs_write(const char *path, const char *data) { return pal_fs_write(path, data); }
int __kioto_fs_append(const char *path, const char *data) { return pal_fs_append(path, data); }
int64_t __kioto_fs_exists(const char *path) { return pal_fs_exists(path); }
int64_t __kioto_fs_is_dir(const char *path) { return pal_fs_is_dir(path); }
int64_t __kioto_fs_size(const char *path) { return pal_fs_size(path); }
int __kioto_fs_copy(const char *src, const char *dst) { return pal_fs_copy(src, dst); }
int __kioto_fs_move(const char *src, const char *dst) { return pal_fs_move(src, dst); }
int __kioto_fs_delete(const char *path) { return pal_fs_delete(path); }
void *__kioto_fs_list(const char *path) { return pal_fs_list(path); }
int __kioto_fs_mkdir(const char *path) { return pal_fs_mkdir(path); }
int __kioto_fs_rmdir(const char *path) { return pal_fs_rmdir(path); }
char *__kioto_fs_join(const char *a, const char *b) { return pal_fs_join(a, b); }
char *__kioto_fs_dirname(const char *path) { return pal_fs_dir(path); }
char *__kioto_fs_basename(const char *path) { return pal_fs_name(path); }
char *__kioto_fs_extension(const char *path) { return pal_fs_ext(path); }

// ── Env ──
char *__kioto_env_get(const char *key) { return pal_env_get(key); }
int __kioto_env_set(const char *key, const char *value) { return pal_env_set(key, value); }
void *__kioto_env_all(void) { return pal_env_all(); }
void *__kioto_env_args(void) { extern void *mire_get_args(int, char**); return mire_get_args(0, NULL); }
char *__kioto_env_cwd(void) { return pal_env_cwd(); }
int __kioto_env_chdir(const char *path) { return pal_env_chdir(path); }

// ── Strings ──
char *__kioto_strings_to_upper(const char *s) { return rt_string_to_upper(s); }
char *__kioto_strings_to_lower(const char *s) { return rt_string_to_lower(s); }
char *__kioto_strings_trim(const char *s) { return mire_strings_trim(s); }
char *__kioto_strings_strip(const char *s) { return mire_strings_trim(s); }
char *__kioto_strings_replace(const char *s, const char *old, const char *rep) { return mire_strings_replace(s, old, rep); }
char *__kioto_strings_replace_first(const char *s, const char *old, const char *rep) { return mire_strings_replace_first(s, old, rep); }
int64_t __kioto_strings_contains(const char *s, const char *sub) { return mire_strings_contains(s, sub); }
int64_t __kioto_strings_starts_with(const char *s, const char *prefix) { return mire_strings_starts_with(s, prefix); }
int64_t __kioto_strings_ends_with(const char *s, const char *suffix) { return mire_strings_ends_with(s, suffix); }
int64_t __kioto_strings_len(const char *s) { return s ? (int64_t)strlen(s) : 0; }
void *__kioto_strings_split(const char *s, const char *sep) {
    // Returns a managed list of strings
    void *list = rt_list_create(8, 8);
    if (!s || !sep || !*sep) { list = rt_list_push_ptr(list, rt_managed_from_slice(s ? s : "", s ? strlen(s) : 0)); return list; }
    char *copy = rt_strdup_raw(s);
    if (!copy) return list;
    char *token = strtok(copy, sep);
    while (token) {
        list = rt_list_push_ptr(list, rt_managed_from_slice(token, strlen(token)));
        token = strtok(NULL, sep);
    }
    free(copy);
    return list;
}
char *__kioto_strings_join(void *parts, const char *sep) {
    int64_t count = rt_list_len(parts);
    size_t dlen = sep ? strlen(sep) : 0;
    size_t total = 0;
    for (int64_t i = 0; i < count; i++) {
        char *s = rt_list_get_ptr(parts, i);
        if (s) total += strlen(s);
    }
    total += dlen * (count > 0 ? count - 1 : 0);
    char *out = rt_managed_alloc(total);
    if (!out) return rt_managed_from_slice("", 0);
    size_t pos = 0;
    for (int64_t i = 0; i < count; i++) {
        if (i > 0 && dlen > 0) { memcpy(out + pos, sep, dlen); pos += dlen; }
        char *s = rt_list_get_ptr(parts, i);
        if (s) {
            size_t slen = strlen(s);
            memcpy(out + pos, s, slen);
            pos += slen;
        }
    }
    out[pos] = '\0';
    return out;
}
char *__kioto_strings_substr(const char *s, int64_t start, int64_t len) { return mire_strings_substr(s, start, len); }
char *__kioto_strings_pad_left(const char *s, int64_t w, const char *pad) { return mire_strings_pad_left(s, w, pad); }
char *__kioto_strings_pad_right(const char *s, int64_t w, const char *pad) { return mire_strings_pad_right(s, w, pad); }
// ── Strings (extra builtins called from LLVM codegen) ──
void *mire_strings_split_list(const char *input, const char *delimiter) {
    if (input == NULL || delimiter == NULL) return rt_list_create(0, 8);
    size_t delim_len = strlen(delimiter);
    size_t input_len = strlen(input);
    if (delim_len == 0) {
        void *list = rt_list_create(1, 8);
        if (!list) return NULL;
        char *copy = rt_strdup_raw(input);
        if (copy) list = rt_list_push_ptr(list, copy);
        return list;
    }
    size_t count = 1;
    const char *cursor = input;
    const char *match;
    while ((match = strstr(cursor, delimiter)) != NULL) { count++; cursor = match + delim_len; }
    void *list = rt_list_create((int64_t)count, 8);
    if (!list) return NULL;
    const char *seg_start = input;
    cursor = input;
    while ((match = strstr(cursor, delimiter)) != NULL) {
        size_t seg_len = (size_t)(match - seg_start);
        char *copy = rt_strdup_raw_n(seg_start, seg_len);
        if (copy) list = rt_list_push_ptr(list, copy);
        seg_start = match + delim_len;
        cursor = seg_start;
    }
    size_t tail_len = input_len - (size_t)(seg_start - input);
    char *tail = rt_strdup_raw_n(seg_start, tail_len);
    if (tail) list = rt_list_push_ptr(list, tail);
    return list;
}

// ── Math ──
static double mire_double_round(double x) { return x >= 0.0 ? floor(x + 0.5) : ceil(x - 0.5); }
static double mire_double_floor(double x) { return floor(x); }
static double mire_double_ceil(double x) { return ceil(x); }
int64_t __kioto_math_round(double x) { return (int64_t)mire_double_round(x); }
int64_t __kioto_math_floor(double x) { return (int64_t)mire_double_floor(x); }
int64_t __kioto_math_ceil(double x) { return (int64_t)mire_double_ceil(x); }
int64_t __kioto_math_abs(int64_t x) { return x < 0 ? -x : x; }
int64_t __kioto_math_sum(void *list) {
    int64_t len = rt_list_len(list);
    int64_t sum = 0;
    for (int64_t i = 0; i < len; i++) sum += rt_list_get_i64(list, i);
    return sum;
}

// ── Time ──
int64_t __kioto_time_mark(void) { return pal_time_mark(); }
int64_t __kioto_time_elapsed_ms(int64_t start) { return pal_time_elapsed_ms(start); }
int64_t __kioto_time_elapsed_ns(int64_t start) { return pal_time_elapsed_ns(start); }
char *__kioto_time_elapsed_ms_str(int64_t start_ns) {
    int64_t ms = pal_time_elapsed_ms(start_ns);
    return rt_managed_printf_i64("%lld", (long long)ms);
}
int64_t __kioto_time_unix_ms(void) { return pal_time_unix_ms(); }
int64_t __kioto_time_unix_ns(void) { return pal_time_unix_ns(); }
int64_t __kioto_time_since_ms(int64_t start_ns) { return pal_time_since_ms(start_ns); }
int64_t __kioto_time_since_ns(int64_t start_ns) { return pal_time_since_ns(start_ns); }
void __kioto_time_sleep_ms(int64_t ms) { pal_time_sleep_ms(ms); }
void __kioto_time_sleep_ns(int64_t ns) { pal_time_sleep_ns(ns); }

// ── CPU ──
int64_t __kioto_cpu_mark(void) { return pal_cpu_mark(); }
int64_t __kioto_cpu_elapsed_ms(int64_t start) { return pal_cpu_elapsed_ms(start); }
int64_t __kioto_cpu_elapsed_ns(int64_t start) { return pal_cpu_elapsed_ns(start); }
int64_t __kioto_cpu_cycles_est(int64_t start) { return pal_cpu_cycles_est(start); }
char *__kioto_cpu_elapsed_ms_str(int64_t start_ns) {
    int64_t ms = pal_cpu_elapsed_ms(start_ns);
    return rt_managed_printf_i64("%lld", (long long)ms);
}
int64_t __kioto_cpu_count(void) { return pal_cpu_count(); }
int64_t __kioto_cpu_freq_mhz(void) { return pal_cpu_freq_mhz(); }
int64_t __kioto_cpu_time_ns(void) { return pal_cpu_time_ns(); }
int64_t __kioto_cpu_time_ms(void) { return pal_cpu_time_ms(); }
void *__kioto_cpu_loadavg(void) { return pal_cpu_loadavg(); }
void *__kioto_cpu_snapshot(void) { return pal_cpu_snapshot(); }

// ── Proc ──
char *__kioto_proc_run(const char *cmd) { return pal_proc_run(cmd); }
char *__kioto_proc_exec(const char *cmd) { return pal_proc_exec(cmd); }
char *__kioto_proc_shell(const char *cmd) { return pal_proc_shell(cmd); }
int64_t __kioto_proc_wait(int64_t pid) { return pal_proc_wait(pid); }
int __kioto_proc_kill(int64_t pid) { return pal_proc_kill(pid); }
void __kioto_proc_exit(int64_t status) { pal_proc_exit(status); }
int64_t __kioto_proc_exists(int64_t pid) { return pal_proc_exists(pid); }

// ── Lists ──
int64_t __kioto_lists_len(void *list) { return rt_list_len(list); }
int64_t __kioto_lists_get_i64(void *list, int64_t index) { return rt_list_get_i64(list, index); }
void *__kioto_lists_get_ptr(void *list, int64_t index) { return rt_list_get_ptr(list, index); }
void *__kioto_lists_push_i64(void *list, int64_t value) { return rt_list_push_i64(list, value); }
void *__kioto_lists_push_ptr(void *list, void *value) { return rt_list_push_ptr(list, value); }
int64_t __kioto_lists_pop(void *list) { return rt_list_pop_i64(list); }
void *__kioto_lists_slice(void *list, int64_t start, int64_t end) { return rt_list_slice(list, start, end); }
void *__kioto_lists_concat(void *a, void *b) { return rt_list_concat(a, b); }
void *__kioto_lists_remove(void *list, int64_t index) { return rt_list_remove(list, index); }
void *__kioto_lists_clear(void *list) { return rt_list_clear(list); }
void *__kioto_lists_flatten(void *list) {
    void *result = rt_list_create(8, 8);
    int64_t len = rt_list_len(list);
    for (int64_t i = 0; i < len; i++) {
        void *sublist = rt_list_get_ptr(list, i);
        int64_t sublen = rt_list_len(sublist);
        for (int64_t j = 0; j < sublen; j++)
            result = rt_list_push_i64(result, rt_list_get_i64(sublist, j));
    }
    return result;
}
void *__kioto_lists_sort(void *list) {
    int64_t len = rt_list_len(list);
    int64_t *arr = (int64_t *)list + 1;
    for (int64_t i = 1; i < len; i++) {
        int64_t key = arr[i];
        int64_t j = i - 1;
        while (j >= 0 && arr[j] > key) { arr[j + 1] = arr[j]; j--; }
        arr[j + 1] = key;
    }
    return list;
}
char *__kioto_lists_join(void *list, const char *sep) {
    int64_t count = rt_list_len(list);
    return __kioto_strings_join(list, sep);
}
int64_t __kioto_lists_first(void *list) {
    if (rt_list_len(list) <= 0) return 0;
    return rt_list_get_i64(list, 0);
}
int64_t __kioto_lists_last(void *list) {
    int64_t len = rt_list_len(list);
    if (len <= 0) return 0;
    return rt_list_get_i64(list, len - 1);
}
// ── Dicts ──
int64_t __kioto_dicts_len(void *dict) { return rt_dict_len(dict); }
void *__kioto_dicts_get(void *dict, const char *key) {
    return rt_dict_get_ptr(dict, 3, 0, (void *)key, NULL);
}
void *__kioto_dicts_set(void *dict, const char *key, void *value) {
    return rt_dict_set_ptr(dict, 3, dict ? ((MireDict *)dict)->value_kind : 3, 0, (void *)key, value);
}
void *__kioto_dicts_set_i64(void *dict, const char *key, int64_t value) {
    return rt_dict_set_i64(dict, 3, 1, 0, (void *)key, value);
}
int64_t __kioto_dicts_has(void *dict, const char *key) {
    return rt_dict_has(dict, 3, 0, (void *)key);
}
void *__kioto_dicts_remove(void *dict, const char *key) {
    return rt_dict_remove(dict, 3, 0, (void *)key);
}
void *__kioto_dicts_keys(void *dict) { return rt_dict_keys(dict); }
void *__kioto_dicts_values(void *dict) { return rt_dict_values(dict); }
int64_t __kioto_dicts_entries(void *dict) { return rt_dict_len(dict); }
void *__kioto_dicts_merge(void *a, void *b) {
    MireDict *dict_b = (MireDict *)b;
    if (!dict_b) return a;
    for (int64_t i = 0; i < dict_b->len; i++) {
        const void *key_slot_ptr = dict_b->key_storage + i * dict_b->key_size;
        if (dict_b->key_kind == MIRE_KIND_STR) {
            const char *k = *(const char **)key_slot_ptr;
            const void *value_slot_ptr = dict_b->value_storage + i * dict_b->value_size;
            if (dict_b->value_kind == MIRE_KIND_STR || dict_b->value_kind == MIRE_KIND_MAP || dict_b->value_kind == MIRE_KIND_PTR) {
                void *v = *(void **)value_slot_ptr;
                a = rt_dict_set_ptr(a, MIRE_KIND_STR, dict_b->value_kind, 0, (void *)k, v);
            } else {
                int64_t v = *(int64_t *)value_slot_ptr;
                a = rt_dict_set_i64(a, MIRE_KIND_STR, MIRE_KIND_SCALAR, 0, (void *)k, v);
            }
        } else {
            int64_t k;
            switch (dict_b->key_size) {
                case 1: k = *(uint8_t *)key_slot_ptr; break;
                case 2: k = *(uint16_t *)key_slot_ptr; break;
                case 4: k = *(uint32_t *)key_slot_ptr; break;
                default: k = *(int64_t *)key_slot_ptr; break;
            }
            const void *value_slot_ptr = dict_b->value_storage + i * dict_b->value_size;
            if (dict_b->value_kind == MIRE_KIND_STR || dict_b->value_kind == MIRE_KIND_MAP || dict_b->value_kind == MIRE_KIND_PTR) {
                void *v = *(void **)value_slot_ptr;
                a = rt_dict_set_ptr(a, dict_b->key_kind, dict_b->value_kind, k, NULL, v);
            } else {
                int64_t v;
                switch (dict_b->value_size) {
                    case 1: v = *(uint8_t *)value_slot_ptr; break;
                    case 2: v = *(uint16_t *)value_slot_ptr; break;
                    case 4: v = *(uint32_t *)value_slot_ptr; break;
                    default: v = *(int64_t *)value_slot_ptr; break;
                }
                a = rt_dict_set_i64(a, dict_b->key_kind, MIRE_KIND_SCALAR, k, NULL, v);
            }
        }
    }
    return a;
}
int64_t __kioto_dicts_is_empty(void *dict) { return rt_dict_len(dict) <= 0 ? 1 : 0; }

// ── Mem ──
int64_t __kioto_mem_process_bytes(void) { return pal_mem_process_bytes(); }
char *__kioto_mem_format(int64_t bytes) { return pal_mem_format(bytes); }
int64_t __kioto_mem_used(void) { return pal_mem_used(); }
int64_t __kioto_mem_total(void) { return pal_mem_total(); }
int64_t __kioto_mem_free(void) { return pal_mem_free(); }
int64_t __kioto_mem_available(void) { return pal_mem_available(); }
int64_t __kioto_mem_percent(void) { return pal_mem_percent(); }
void *__kioto_mem_snapshot(void) { return pal_mem_snapshot(); }

// ── IO ──
void __kioto_io_print(const char *msg) { if (msg) printf("%s", msg); }
void __kioto_io_print_err(const char *msg) { if (msg) fprintf(stderr, "%s", msg); }
char *__kioto_io_readln(void) { return mire_read_line(NULL); }

// ── GPU ──
char *__kioto_gpu_snapshot(void) { return pal_gpu_snapshot(); }

// ── Term ──
char *__kioto_term_style(const char *s, const char *style) { return pal_term_style(s, style); }
char *__kioto_term_hr(const char *ch, int64_t len) { return pal_term_hr(ch, len); }
char *__kioto_term_clear(void) { return pal_term_clear(); }

// ── Unreferenced static helpers used by dicts ──
// These are intentionally re-exported from dicts.c via the header
