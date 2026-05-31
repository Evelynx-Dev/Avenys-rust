// WASM PAL — I/O stubs
// WASM console I/O is limited under WASI.

#include "pal.h"
#include "runtime.h"
#include <stdio.h>

void pal_io_print(const char *msg) {
    if (msg) printf("%s", msg);
}

void pal_io_print_err(const char *msg) {
    if (msg) fprintf(stderr, "%s", msg);
}

char *pal_io_readln(void) {
    return rt_managed_from_slice("", 0);
}
