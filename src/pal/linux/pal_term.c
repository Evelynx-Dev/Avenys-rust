#include "../pal.h"
#include "runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *pal_term_style(const char *text, const char *style) {
    const char *code = "";
    if (strcmp(style, "bold") == 0) code = "1";
    else if (strcmp(style, "dim") == 0) code = "2";
    else if (strcmp(style, "italic") == 0) code = "3";
    else if (strcmp(style, "underline") == 0) code = "4";
    else if (strcmp(style, "blink") == 0) code = "5";
    else if (strcmp(style, "reverse") == 0) code = "7";
    else if (strcmp(style, "red") == 0) code = "31";
    else if (strcmp(style, "green") == 0) code = "32";
    else if (strcmp(style, "yellow") == 0) code = "33";
    else if (strcmp(style, "blue") == 0) code = "34";
    else if (strcmp(style, "magenta") == 0) code = "35";
    else if (strcmp(style, "cyan") == 0) code = "36";
    else if (strcmp(style, "white") == 0) code = "37";
    else if (strcmp(style, "reset") == 0) code = "0";

    size_t tlen = text ? strlen(text) : 0;
    size_t clen = strlen(code);
    // \033[<code>m<text>\033[0m
    char *out = (char *)malloc(clen + tlen + 9);
    if (!out) return rt_managed_from_slice("", 0);
    int n = snprintf(out, clen + tlen + 9, "\033[%sm%s\033[0m", code, text ? text : "");
    (void)n;
    char *result = rt_managed_from_slice(out, strlen(out));
    free(out);
    return result;
}

char *pal_term_hr(const char *ch, int64_t len) {
    if (!ch || !*ch || len <= 0) return rt_managed_from_slice("", 0);
    char c = ch[0];
    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) return rt_managed_from_slice("", 0);
    memset(buf, c, (size_t)len);
    buf[len] = '\0';
    char *result = rt_managed_from_slice(buf, (size_t)len);
    free(buf);
    return result;
}

char *pal_term_clear(void) {
    return rt_managed_from_slice("\033[2J\033[H", 7);
}
