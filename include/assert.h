#ifndef _ASSERT_H
#define _ASSERT_H

extern void abort(void);
extern int fprintf(void *stream, const char *format, ...);
extern void *stderr;

#ifdef NDEBUG
#define assert(expr) ((void)0)
#else
#define assert(expr) ((expr) ? (void)0 : (abort(), (void)0))
#endif

#endif
