#ifndef _STDIO_H
#define _STDIO_H

#include <stddef.h>
#include <stdarg.h>

typedef struct _IO_FILE FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

extern int printf(const char *format, ...);
extern int fprintf(FILE *stream, const char *format, ...);
extern int sprintf(char *str, const char *format, ...);
extern int snprintf(char *str, size_t size, const char *format, ...);
extern int vprintf(const char *format, va_list ap);
extern int vfprintf(FILE *stream, const char *format, va_list ap);
extern int vsprintf(char *str, const char *format, va_list ap);
extern int vsnprintf(char *str, size_t size, const char *format, va_list ap);
extern int scanf(const char *format, ...);
extern int fscanf(FILE *stream, const char *format, ...);
extern int sscanf(const char *str, const char *format, ...);

extern int fgetc(FILE *stream);
extern char *fgets(char *s, int size, FILE *stream);
extern int fputc(int c, FILE *stream);
extern int fputs(const char *s, FILE *stream);
extern int getc(FILE *stream);
extern int getchar(void);
extern int putc(int c, FILE *stream);
extern int putchar(int c);
extern int puts(const char *s);
extern int ungetc(int c, FILE *stream);

extern size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream);
extern size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream);

extern FILE *fopen(const char *pathname, const char *mode);
extern FILE *fdopen(int fd, const char *mode);
extern FILE *freopen(const char *pathname, const char *mode, FILE *stream);
extern int fclose(FILE *stream);
extern int fflush(FILE *stream);

extern int fseek(FILE *stream, long offset, int whence);
extern long ftell(FILE *stream);
extern void rewind(FILE *stream);
extern int feof(FILE *stream);
extern int ferror(FILE *stream);
extern void clearerr(FILE *stream);

extern int fileno(FILE *stream);
extern int remove(const char *pathname);
extern int rename(const char *oldpath, const char *newpath);
extern FILE *tmpfile(void);
extern char *tmpnam(char *s);
extern void perror(const char *s);

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
#define EOF (-1)
#define BUFSIZ 8192

#endif
