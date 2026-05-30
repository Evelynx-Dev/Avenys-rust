#include "../pal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
#include <dirent.h>

int pal_fs_write(const char *path, const char *content) {
    FILE *fh = fopen(path, "w");
    if (!fh) return 0;
    int ok = fputs(content, fh) >= 0;
    fclose(fh);
    return ok ? 1 : 0;
}

int pal_fs_append(const char *path, const char *content) {
    FILE *fh = fopen(path, "a");
    if (!fh) return 0;
    int ok = fputs(content, fh) >= 0;
    fclose(fh);
    return ok ? 1 : 0;
}

char *pal_fs_read(const char *path) {
    FILE *fh = fopen(path, "rb");
    if (!fh) return NULL;
    fseek(fh, 0, SEEK_END);
    long size = ftell(fh);
    fseek(fh, 0, SEEK_SET);
    char *buf = (char *)malloc((size_t)(size + 1));
    if (!buf) { fclose(fh); return NULL; }
    size_t read = fread(buf, 1, (size_t)size, fh);
    buf[read] = '\0';
    fclose(fh);
    return buf;
}

int pal_fs_copy(const char *src, const char *dst) {
    FILE *in = fopen(src, "rb");
    if (!in) return 0;
    FILE *out = fopen(dst, "wb");
    if (!out) { fclose(in); return 0; }
    char buf[4096];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), in)) > 0)
        fwrite(buf, 1, n, out);
    fclose(in);
    fclose(out);
    return 1;
}

int pal_fs_move(const char *src, const char *dst) {
    if (rename(src, dst) == 0) return 1;
    if (pal_fs_copy(src, dst)) {
        remove(src);
        return 1;
    }
    return 0;
}

int pal_fs_delete(const char *path) {
    return remove(path) == 0 ? 1 : 0;
}

int pal_fs_mkdir(const char *path) {
    return mkdir(path, 0755) == 0 ? 1 : 0;
}

int pal_fs_rmdir(const char *path) {
    return rmdir(path) == 0 ? 1 : 0;
}

int64_t pal_fs_exists(const char *path) {
    return access(path, F_OK) == 0 ? 1 : 0;
}

int64_t pal_fs_is_dir(const char *path) {
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISDIR(st.st_mode) ? 1 : 0;
}

int64_t pal_fs_size(const char *path) {
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return (int64_t)st.st_size;
}

void *pal_fs_list(const char *path) {
    // returns a raw list of char* pointers (not managed)
    extern void *rt_list_create(int64_t initial_cap, int64_t elem_size);
    extern int64_t rt_list_len(void *list_ptr);
    extern void *rt_list_push_ptr(void *list_ptr, void *value);
    extern char *rt_strdup_raw(const char *src);

    void *list = rt_list_create(16, 8);
    DIR *dir = opendir(path);
    if (!dir) return list;
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        char *name = rt_strdup_raw(entry->d_name);
        list = rt_list_push_ptr(list, name);
    }
    closedir(dir);
    return list;
}

char *pal_fs_join(const char *a, const char *b) {
    size_t alen = strlen(a);
    size_t blen = strlen(b);
    char *out = (char *)malloc(alen + blen + 2);
    if (!out) return NULL;
    snprintf(out, alen + blen + 2, "%s/%s", a, b);
    return out;
}

char *pal_fs_dir(const char *path) {
    const char *slash = strrchr(path, '/');
    if (!slash) return NULL;
    size_t len = (size_t)(slash - path);
    char *out = (char *)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, path, len);
    out[len] = '\0';
    return out;
}

char *pal_fs_name(const char *path) {
    const char *slash = strrchr(path, '/');
    const char *base = slash ? slash + 1 : path;
    return strdup(base);
}

char *pal_fs_ext(const char *path) {
    const char *dot = strrchr(path, '.');
    if (!dot || dot == path) return NULL;
    return strdup(dot);
}
