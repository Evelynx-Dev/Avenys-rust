// WASM PAL — Terminal stubs
// WASM has no terminal support.

#include "pal.h"
#include <stdlib.h>
#include <string.h>

char *pal_term_style(const char *text, const char *style) {
    (void)style;
    return strdup(text);
}

char *pal_term_hr(const char *ch, int64_t len) {
    char *buf = malloc((size_t)len + 1);
    if (!buf) return NULL;
    for (int64_t i = 0; i < len; i++) buf[i] = ch[0];
    buf[len] = '\0';
    return buf;
}

char *pal_term_clear(void) {
    return NULL;
}
