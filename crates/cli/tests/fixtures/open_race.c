#define _GNU_SOURCE
#include <fcntl.h>
#include <errno.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

ssize_t write(int fd, const void *buffer, size_t count) {
    const char *target = getenv("AGENT_TEST_FAIL_WRITE");
    if (target) {
        char descriptor[64];
        char path[4096];
        snprintf(descriptor, sizeof(descriptor), "/proc/self/fd/%d", fd);
        ssize_t length = readlink(descriptor, path, sizeof(path) - 1);
        if (length >= 0) {
            path[length] = '\0';
            if (strcmp(path, target) == 0) {
                errno = EFBIG;
                return -1;
            }
        }
    }
    return syscall(SYS_write, fd, buffer, count);
}

int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, int);
        va_end(args);
    }
    const char *target = getenv("AGENT_TEST_REPLACE_INPUT");
    const char *blocked = getenv("AGENT_TEST_BLOCK_INPUT");
    const char *notify = getenv("AGENT_TEST_BLOCK_NOTIFY_FD");
    if (blocked && notify && strcmp(path, blocked) == 0) {
        if (write(atoi(notify), "1", 1) != 1) {
            _exit(125);
        }
        for (;;) {
            pause();
        }
    }
    const char *backup = getenv("AGENT_TEST_ORIGINAL_INPUT");
    if (target && backup && strcmp(path, target) == 0) {
        if (rename(target, backup) != 0) {
            _exit(125);
        }
        const char *kind = getenv("AGENT_TEST_REPLACEMENT_KIND");
        const char *replacement = getenv("AGENT_TEST_REPLACEMENT_FILE");
        int result;
        if (kind && strcmp(kind, "fifo") == 0) {
            result = mkfifo(target, 0600);
        } else if (kind && replacement && strcmp(kind, "file") == 0) {
            result = link(replacement, target);
        } else if (kind && replacement && strcmp(kind, "symlink") == 0) {
            result = symlink(replacement, target);
        } else {
            result = mkdir(target, 0700);
        }
        if (result != 0) {
            _exit(125);
        }
    }
    return syscall(SYS_openat, AT_FDCWD, path, flags, mode);
}

int fsync(int fd) {
    const char *target = getenv("AGENT_TEST_BLOCK_SYNC");
    if (target) {
        char descriptor[64];
        char path[4096];
        snprintf(descriptor, sizeof(descriptor), "/proc/self/fd/%d", fd);
        ssize_t length = readlink(descriptor, path, sizeof(path) - 1);
        if (length >= 0) {
            path[length] = '\0';
            if (strcmp(path, target) == 0) {
                const char *notify = getenv("AGENT_TEST_SYNC_NOTIFY");
                const char *release = getenv("AGENT_TEST_SYNC_RELEASE");
                char value;
                if (!notify || !release || write(atoi(notify), "1", 1) != 1 || read(atoi(release), &value, 1) != 1) {
                    _exit(125);
                }
            }
        }
    }
    return syscall(SYS_fsync, fd);
}
