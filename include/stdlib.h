#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>

extern void *malloc(size_t size);
extern void *calloc(size_t nmemb, size_t size);
extern void *realloc(void *ptr, size_t size);
extern void free(void *ptr);

extern void abort(void);
extern void exit(int status);
extern void _exit(int status);
extern int atexit(void (*function)(void));

extern int atoi(const char *nptr);
extern long atol(const char *nptr);
extern long long atoll(const char *nptr);
extern double atof(const char *nptr);
extern long strtol(const char *nptr, char **endptr, int base);
extern unsigned long strtoul(const char *nptr, char **endptr, int base);
extern long long strtoll(const char *nptr, char **endptr, int base);
extern unsigned long long strtoull(const char *nptr, char **endptr, int base);
extern double strtod(const char *nptr, char **endptr);

extern int abs(int j);
extern long labs(long j);

extern void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
extern void *bsearch(const void *key, const void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));

extern int rand(void);
extern void srand(unsigned int seed);

extern char *getenv(const char *name);
extern int setenv(const char *name, const char *value, int overwrite);
extern int unsetenv(const char *name);
extern int system(const char *command);

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
#define RAND_MAX 2147483647

#endif
