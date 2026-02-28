#ifndef _UNISTD_H
#define _UNISTD_H

#include <stddef.h>

typedef long ssize_t;
typedef int pid_t;
typedef unsigned int uid_t;
typedef unsigned int gid_t;
typedef long off_t;

extern ssize_t read(int fd, void *buf, size_t count);
extern ssize_t write(int fd, const void *buf, size_t count);
extern int close(int fd);
extern off_t lseek(int fd, off_t offset, int whence);
extern int dup(int oldfd);
extern int dup2(int oldfd, int newfd);
extern int pipe(int pipefd[2]);
extern pid_t fork(void);
extern pid_t getpid(void);
extern pid_t getppid(void);
extern uid_t getuid(void);
extern gid_t getgid(void);
extern int execv(const char *pathname, char *const argv[]);
extern int execvp(const char *file, char *const argv[]);
extern int execve(const char *pathname, char *const argv[], char *const envp[]);
extern unsigned int sleep(unsigned int seconds);
extern int usleep(unsigned int usec);
extern int unlink(const char *pathname);
extern int rmdir(const char *pathname);
extern int chdir(const char *path);
extern char *getcwd(char *buf, size_t size);
extern int access(const char *pathname, int mode);
extern int isatty(int fd);
extern long sysconf(int name);
extern int fsync(int fd);
extern int ftruncate(int fd, off_t length);
extern int symlink(const char *target, const char *linkpath);
extern ssize_t readlink(const char *pathname, char *buf, size_t bufsiz);

#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

#define F_OK 0
#define R_OK 4
#define W_OK 2
#define X_OK 1

#endif
