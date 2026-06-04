// WASM PAL — Filesystem stubs
// WASM has no real filesystem unless WASI is available.
// These stubs return errors or empty results.

#include "pal.h"
#include <stdlib.h>
#include <string.h>

int pal_fs_write(const char *path, const char *content) { (void)path; (void)content; return -1; }
int pal_fs_append(const char *path, const char *content) { (void)path; (void)content; return -1; }
char *pal_fs_read(const char *path) { (void)path; return NULL; }
int pal_fs_copy(const char *src, const char *dst) { (void)src; (void)dst; return -1; }
int pal_fs_move(const char *src, const char *dst) { (void)src; (void)dst; return -1; }
int pal_fs_delete(const char *path) { (void)path; return -1; }
int pal_fs_mkdir(const char *path) { (void)path; return -1; }
int pal_fs_rmdir(const char *path) { (void)path; return -1; }
int64_t pal_fs_exists(const char *path) { (void)path; return 0; }
int64_t pal_fs_is_dir(const char *path) { (void)path; return 0; }
int64_t pal_fs_size(const char *path) { (void)path; return -1; }
void *pal_fs_list(const char *path) { (void)path; return NULL; }

char *pal_fs_join(const char *a, const char *b) {
    size_t alen = strlen(a);
    size_t blen = strlen(b);
    char *result = malloc(alen + blen + 2);
    if (!result) return NULL;
    memcpy(result, a, alen);
    result[alen] = '/';
    memcpy(result + alen + 1, b, blen);
    result[alen + blen + 1] = '\0';
    return result;
}

char *pal_fs_dir(const char *path) {
    const char *slash = strrchr(path, '/');
    if (!slash) {
        char *dot = malloc(2);
        if (!dot) return NULL;
        dot[0] = '.';
        dot[1] = '\0';
        return dot;
    }
    size_t len = slash - path;
    char *result = malloc(len + 1);
    if (!result) return NULL;
    memcpy(result, path, len);
    result[len] = '\0';
    return result;
}

char *pal_fs_name(const char *path) {
    const char *slash = strrchr(path, '/');
    const char *start = slash ? slash + 1 : path;
    size_t len = strlen(start);
    char *result = malloc(len + 1);
    if (!result) return NULL;
    memcpy(result, start, len + 1);
    return result;
}

char *pal_fs_ext(const char *path) {
    const char *dot = strrchr(path, '.');
    if (!dot) {
        char *empty = malloc(1);
        if (!empty) return NULL;
        empty[0] = '\0';
        return empty;
    }
    return pal_fs_name(dot);
}
