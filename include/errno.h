#ifndef _ERRNO_H
#define _ERRNO_H

extern int *__errno_location(void);
#define errno (*__errno_location())

#define EPERM 1
#define ENOENT 2
#define ESRCH 3
#define EINTR 4
#define EIO 5
#define ENOMEM 12
#define EACCES 13
#define EEXIST 17
#define ENOTDIR 20
#define EISDIR 21
#define EINVAL 22
#define ENOSPC 28
#define EPIPE 32
#define ERANGE 34
#define EAGAIN 11
#define EWOULDBLOCK EAGAIN
#define ENOTEMPTY 39
#define ENOSYS 38
#define ECONNREFUSED 111
#define ETIMEDOUT 110

#endif
