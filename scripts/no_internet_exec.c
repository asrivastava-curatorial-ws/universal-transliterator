/*
 * Release-test harness: run a command under a seccomp filter that terminates
 * the child if it attempts to create an IPv4 or IPv6 socket.  The harness
 * closes inherited descriptors first, so the command cannot reuse an
 * Internet socket supplied by its parent.
 *
 * This is test tooling only; it is not included in the AppImage.
 */
#define _GNU_SOURCE

#include <errno.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#if !defined(__linux__) || !defined(__x86_64__)
#error "no_internet_exec currently supports Linux x86-64 only"
#endif

static void close_inherited_descriptors(void) {
#ifdef SYS_close_range
    if (syscall(SYS_close_range, 3U, ~0U, 0U) == 0) {
        return;
    }
    if (errno != ENOSYS && errno != EINVAL) {
        perror("close_range");
        _exit(120);
    }
#endif

    struct rlimit limit;
    if (getrlimit(RLIMIT_NOFILE, &limit) != 0) {
        perror("getrlimit");
        _exit(120);
    }
    rlim_t maximum = limit.rlim_cur;
    if (maximum == RLIM_INFINITY || maximum > 1048576U) {
        maximum = 1048576U;
    }
    for (int descriptor = 3; (rlim_t)descriptor < maximum; descriptor++) {
        (void)close(descriptor);
    }
}

static void install_filter(void) {
    struct sock_filter instructions[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS,
                 (unsigned int)offsetof(struct seccomp_data, arch)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS,
                 (unsigned int)offsetof(struct seccomp_data, nr)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, __NR_socket, 0, 4),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS,
                 (unsigned int)offsetof(struct seccomp_data, args[0])),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, AF_INET, 1, 0),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, AF_INET6, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    };
    struct sock_fprog program = {
        .len = (unsigned short)(sizeof(instructions) / sizeof(instructions[0])),
        .filter = instructions,
    };

    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) {
        perror("PR_SET_NO_NEW_PRIVS");
        _exit(121);
    }
    if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program) != 0) {
        perror("PR_SET_SECCOMP");
        _exit(121);
    }
}

static int wait_for_child(pid_t child, const char *description) {
    int status = 0;
    if (waitpid(child, &status, 0) < 0) {
        perror("waitpid");
        return 122;
    }
    if (WIFEXITED(status)) {
        return WEXITSTATUS(status);
    }
    if (WIFSIGNALED(status)) {
        int signal_number = WTERMSIG(status);
        if (signal_number == SIGSYS) {
            fprintf(stderr,
                    "blocked Internet-family socket attempt while running %s\n",
                    description);
            return 123;
        }
        fprintf(stderr, "%s terminated by signal %d\n", description,
                signal_number);
        return 128 + signal_number;
    }
    fprintf(stderr, "%s ended in an unexpected wait state\n", description);
    return 124;
}

static int verify_filter(void) {
    pid_t child = fork();
    if (child < 0) {
        perror("fork");
        return 122;
    }
    if (child == 0) {
        close_inherited_descriptors();
        install_filter();
        (void)socket(AF_INET, SOCK_STREAM, 0);
        _exit(99);
    }

    int status = 0;
    if (waitpid(child, &status, 0) < 0) {
        perror("waitpid");
        return 122;
    }
    if (!WIFSIGNALED(status) || WTERMSIG(status) != SIGSYS) {
        fprintf(stderr, "seccomp self-check did not trap AF_INET socket()\n");
        return 125;
    }
    puts("PASS: seccomp self-check trapped AF_INET socket creation");
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s COMMAND [ARG ...]\n", argv[0]);
        return 2;
    }
    int verification = verify_filter();
    if (verification != 0) {
        return verification;
    }

    pid_t child = fork();
    if (child < 0) {
        perror("fork");
        return 122;
    }
    if (child == 0) {
        close_inherited_descriptors();
        install_filter();
        execvp(argv[1], &argv[1]);
        perror("execvp");
        _exit(126);
    }

    int result = wait_for_child(child, argv[1]);
    if (result == 0) {
        puts("PASS: command completed without creating an IPv4/IPv6 socket");
    }
    return result;
}
