#include "../pal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>

char *pal_proc_run(const char *cmd) {
    FILE *fh = popen(cmd, "r");
    if (!fh) return NULL;
    size_t cap = 256, len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) { pclose(fh); return NULL; }
    int ch;
    while ((ch = fgetc(fh)) != EOF) {
        if (len + 1 >= cap) {
            cap *= 2;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) break;
            buf = nb;
        }
        buf[len++] = (char)ch;
    }
    buf[len] = '\0';
    pclose(fh);
    return buf;
}

char *pal_proc_exec(const char *cmd) {
    return pal_proc_run(cmd);
}

char *pal_proc_shell(const char *cmd) {
    return pal_proc_run(cmd);
}

int64_t pal_proc_wait(int64_t pid) {
    int status;
    waitpid((pid_t)pid, &status, 0);
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    return -1;
}

int pal_proc_kill(int64_t pid) {
    return kill((pid_t)pid, SIGTERM) == 0 ? 1 : 0;
}

void pal_proc_exit(int64_t status) {
    exit((int)status);
}

int64_t pal_proc_exists(int64_t pid) {
    return kill((pid_t)pid, 0) == 0 ? 1 : 0;
}
