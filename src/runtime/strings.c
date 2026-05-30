#include "runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *rt_string_copy(const char *value) {
    if (value == NULL) return rt_managed_from_slice("", 0);
    return rt_managed_from_slice(value, strlen(value));
}

char *rt_string_concat(const char *left, const char *right) {
    if (left == NULL) left = "";
    if (right == NULL) right = "";
    size_t llen = strlen(left);
    size_t rlen = strlen(right);
    char *out = rt_managed_alloc(llen + rlen);
    if (out == NULL) return rt_managed_from_slice("", 0);
    memcpy(out, left, llen);
    memcpy(out + llen, right, rlen);
    out[llen + rlen] = '\0';
    return out;
}

char *rt_string_append_owned(char *value, const char *suffix) {
    if (value == NULL) return rt_string_copy(suffix);
    if (suffix == NULL) return value;
    size_t vlen = strlen(value);
    size_t slen = strlen(suffix);
    if (rt_managed_contains(value)) {
        MireManagedString *hdr = (MireManagedString *)((char *)value - offsetof(MireManagedString, data));
        size_t needed = vlen + slen;
        if (hdr->cap >= needed) {
            memcpy(value + vlen, suffix, slen);
            value[needed] = '\0';
            hdr->len = needed;
            return value;
        }
    }
    char *result = rt_managed_alloc(vlen + slen);
    if (result == NULL) {
        char *fallback = rt_string_concat(value, suffix);
        if (!rt_managed_contains(value)) free(value);
        return fallback;
    }
    if (vlen > 0) memcpy(result, value, vlen);
    if (slen > 0) memcpy(result + vlen, suffix, slen);
    result[vlen + slen] = '\0';
    if (rt_managed_contains(value)) {
        rt_managed_unregister(value);
        MireManagedString *old_hdr = (MireManagedString *)((char *)value - offsetof(MireManagedString, data));
        free(old_hdr);
    } else {
        free(value);
    }
    return result;
}

char *rt_i64_to_string(int64_t value) {
    return rt_managed_printf_i64("%lld", (long long)value);
}

char *rt_bool_to_string(int64_t value) {
    return rt_managed_from_slice(value ? "true" : "false", value ? 4 : 5);
}

char *rt_f64_to_string(double value) {
    return rt_managed_printf_f64("%g", value);
}

char rt_unicode_to_lower(unsigned char c) {
    if (c >= 'A' && c <= 'Z') return (char)(c + 32);
    if (c >= 0xC0 && c <= 0xD6) return (char)(c + 32);
    if (c >= 0xD8 && c <= 0xDE) return (char)(c + 32);
    return (char)c;
}

char rt_unicode_to_upper(unsigned char c) {
    if (c >= 'a' && c <= 'z') return (char)(c - 32);
    if (c >= 0xE0 && c <= 0xF6) return (char)(c - 32);
    if (c >= 0xF8 && c <= 0xFE) return (char)(c - 32);
    return (char)c;
}

char *rt_string_to_upper(const char *value) {
    if (value == NULL) return rt_managed_from_slice("", 0);
    size_t len = strlen(value);
    char *out = rt_managed_alloc(len);
    if (out == NULL) return rt_managed_from_slice("", 0);
    for (size_t i = 0; i < len; i++) {
        out[i] = rt_unicode_to_upper((unsigned char)value[i]);
    }
    out[len] = '\0';
    return out;
}

char *rt_string_to_lower(const char *value) {
    if (value == NULL) return rt_managed_from_slice("", 0);
    size_t len = strlen(value);
    char *out = rt_managed_alloc(len);
    if (out == NULL) return rt_managed_from_slice("", 0);
    for (size_t i = 0; i < len; i++) {
        out[i] = rt_unicode_to_lower((unsigned char)value[i]);
    }
    out[len] = '\0';
    return out;
}
