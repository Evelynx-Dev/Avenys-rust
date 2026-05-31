// WASM PAL — Environment stubs
// Environment variables are not available in standard WASM.

#include "pal.h"
#include <stdlib.h>
#include <string.h>

char *pal_env_get(const char *name) {
    (void)name;
    return NULL;
}

int pal_env_set(const char *name, const char *value) {
    (void)name;
    (void)value;
    return -1;
}

void *pal_env_all(void) {
    return NULL;
}

char *pal_env_cwd(void) {
    char *dot = malloc(2);
    if (!dot) return NULL;
    dot[0] = '.';
    dot[1] = '\0';
    return dot;
}

int pal_env_chdir(const char *path) {
    (void)path;
    return -1;
}
