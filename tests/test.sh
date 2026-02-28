#!/bin/bash

COMPILER=./target/debug/my_own_c_compiler
TMPDIR=$(mktemp -d)
PASS=0
FAIL=0

cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

assert() {
  expected="$1"
  input="$2"

  echo "$input" > "$TMPDIR/tmp.c"
  $COMPILER "$TMPDIR/tmp.c" > "$TMPDIR/tmp.s"
  gcc -o "$TMPDIR/tmp" "$TMPDIR/tmp.s"
  "$TMPDIR/tmp"
  actual="$?"

  if [ "$actual" = "$expected" ]; then
    echo "OK: '$input' => $actual"
    PASS=$((PASS + 1))
  else
    echo "FAIL: '$input' => $actual (expected $expected)"
    FAIL=$((FAIL + 1))
  fi
}

# Step 2.1: return statement and expression statement
assert 0 'int main() { return 0; }'
assert 42 'int main() { return 42; }'
assert 255 'int main() { return 255; }'
assert 3 'int main() { 1; 2; return 3; }'
assert 5 'int main() { return 2+3; }'
assert 47 'int main() { return 5+6*7; }'
assert 15 'int main() { return 5*(9-6); }'
assert 4 'int main() { return (3+5)/2; }'
assert 10 'int main() { return -10+20; }'
assert 10 'int main() { return - -10; }'
assert 10 'int main() { return - - +10; }'
assert 6 'int main() { return 2*3; }'
assert 3 'int main() { return 9/3; }'
assert 20 'int main() { return (2+3)*(5-1); }'
assert 3 'int main() { return 10/2/2+1-1+1; }'
assert 1 'int main() { return 10 % 3; }'
assert 0 'int main() { return 6 % 3; }'
assert 2 'int main() { return 11 % 3; }'
assert 1 'int main() { return 1==1; }'
assert 0 'int main() { return 1==2; }'
assert 1 'int main() { return 1!=2; }'
assert 0 'int main() { return 1!=1; }'
assert 1 'int main() { return 1<2; }'
assert 0 'int main() { return 2<1; }'
assert 0 'int main() { return 1<1; }'
assert 1 'int main() { return 1<=1; }'
assert 1 'int main() { return 1<=2; }'
assert 0 'int main() { return 2<=1; }'
assert 1 'int main() { return 2>1; }'
assert 0 'int main() { return 1>2; }'
assert 1 'int main() { return 2>=2; }'
assert 1 'int main() { return 3>=2; }'
assert 0 'int main() { return 1>=2; }'
assert 1 'int main() { return 5+1==6; }'
assert 1 'int main() { return 3*2>=5; }'

# Step 2.2: local variables (single character)
assert 3 'int main() { int a; a = 3; return a; }'
assert 8 'int main() { int a; int b; a = 3; b = 5; return a + b; }'
assert 14 'int main() { int a; int b; a = 3; b = 5; return a * b - 1; }'

# Step 2.3: multi-character variable names
assert 3 'int main() { int foo; int bar; foo = 1; bar = 2; return foo + bar; }'
assert 10 'int main() { int hello = 10; return hello; }'
assert 14 'int main() { int a_b = 3; int c_d = 5; int e = a_b + c_d; return a_b + c_d + e - 2; }'

# Step 2.4: if statement
assert 1 'int main() { if (1) return 1; return 0; }'
assert 0 'int main() { if (0) return 1; return 0; }'
assert 2 'int main() { if (0) return 1; else return 2; }'
assert 1 'int main() { if (1) return 1; else return 2; }'
assert 4 'int main() { int a = 0; if (1) a = 4; return a; }'

# Step 2.5: while statement
assert 10 'int main() { int i = 0; while (i < 10) i = i + 1; return i; }'
assert 0 'int main() { int i = 0; while (i > 0) i = i - 1; return i; }'

# Step 2.6: for statement
assert 45 'int main() { int s = 0; int i; for (i = 0; i < 10; i = i + 1) s = s + i; return s; }'
assert 10 'int main() { int i = 0; for (;i < 10;) i = i + 1; return i; }'
assert 55 'int main() { int s = 0; int i; for (i = 1; i <= 10; i = i + 1) s = s + i; return s; }'

# Step 2.7: block statement
assert 3 'int main() { { return 3; } }'
assert 5 'int main() { { int a = 2; int b = 3; return a + b; } }'
assert 55 'int main() { int s = 0; int i = 1; while (i <= 10) { s = s + i; i = i + 1; } return s; }'

# Step 2.9: compound assignment operators
assert 15 'int main() { int a = 10; a += 5; return a; }'
assert 5 'int main() { int a = 10; a -= 5; return a; }'
assert 20 'int main() { int a = 10; a *= 2; return a; }'
assert 5 'int main() { int a = 10; a /= 2; return a; }'
assert 1 'int main() { int a = 10; a %= 3; return a; }'

# Step 2.10: increment/decrement
assert 6 'int main() { int a = 5; a++; return a; }'
assert 5 'int main() { int a = 5; return a++; }'
assert 6 'int main() { int a = 5; return ++a; }'
assert 4 'int main() { int a = 5; a--; return a; }'
assert 5 'int main() { int a = 5; return a--; }'
assert 4 'int main() { int a = 5; return --a; }'

# Step 2.11: logical operators
assert 1 'int main() { return 1 && 1; }'
assert 0 'int main() { return 1 && 0; }'
assert 0 'int main() { return 0 && 1; }'
assert 0 'int main() { return 0 && 0; }'
assert 1 'int main() { return 1 || 0; }'
assert 1 'int main() { return 0 || 1; }'
assert 0 'int main() { return 0 || 0; }'
assert 1 'int main() { return 1 || 1; }'
assert 1 'int main() { return !0; }'
assert 0 'int main() { return !1; }'
assert 0 'int main() { return !42; }'
assert 1 'int main() { return 2 && 3; }'
assert 1 'int main() { return 2 || 0; }'

# Step 2.12: bitwise operators
assert 1 'int main() { return 3 & 1; }'
assert 3 'int main() { return 1 | 2; }'
assert 3 'int main() { return 1 ^ 2; }'
assert 0 'int main() { return 3 ^ 3; }'
assert 8 'int main() { return 1 << 3; }'
assert 2 'int main() { return 8 >> 2; }'
assert 5 'int main() { return 7 & 5; }'
assert 7 'int main() { return 5 | 3; }'
assert 253 'int main() { int a = 2; return ~a & 255; }'

# Step 2.13: comma operator and ternary operator
assert 3 'int main() { return (1, 2, 3); }'
assert 10 'int main() { return 1 ? 10 : 20; }'
assert 20 'int main() { return 0 ? 10 : 20; }'
assert 5 'int main() { int a = 3; return (a = 5, a); }'
assert 10 'int main() { int a = 5; return a > 3 ? 10 : 20; }'

# Step 2.14: do-while, switch/case/default, break
assert 5 'int main() { int i = 0; do { i++; } while (i < 5); return i; }'
assert 1 'int main() { int i = 0; do { i++; } while (0); return i; }'
assert 20 'int main() { int a = 2; switch (a) { case 1: return 10; case 2: return 20; default: return 30; } }'
assert 10 'int main() { int a = 1; switch (a) { case 1: return 10; case 2: return 20; default: return 30; } }'
assert 30 'int main() { int a = 9; switch (a) { case 1: return 10; case 2: return 20; default: return 30; } }'
assert 3 'int main() { int i = 0; while (i < 10) { if (i == 3) break; i++; } return i; }'
assert 20 'int main() { int a = 2; int r = 0; switch (a) { case 1: r = 10; break; case 2: r = 20; break; default: r = 30; break; } return r; }'

# Step 3.1: function calls (no args) and multiple function definitions
assert 3 'int ret3() { return 3; } int main() { return ret3(); }'
assert 5 'int ret5() { return 5; } int main() { return ret5(); }'
assert 8 'int ret3() { return 3; } int ret5() { return 5; } int main() { return ret3() + ret5(); }'

# Step 3.2: function arguments (up to 6)
assert 7 'int add(int a, int b) { return a + b; } int main() { return add(3, 4); }'
assert 1 'int sub(int a, int b) { return a - b; } int main() { return sub(4, 3); }'
assert 120 'int fact(int n) { if (n <= 1) return 1; return n * fact(n - 1); } int main() { return fact(5); }'
assert 55 'int fib(int n) { if (n <= 1) return n; return fib(n-1) + fib(n-2); } int main() { return fib(10); }'
assert 21 'int add6(int a, int b, int c, int d, int e, int f) { return a+b+c+d+e+f; } int main() { return add6(1,2,3,4,5,6); }'

# Step 3.3: stack-passed arguments (7+)
assert 36 'int add8(int a, int b, int c, int d, int e, int f, int g, int h) { return a+b+c+d+e+f+g+h; } int main() { return add8(1,2,3,4,5,6,7,8); }'
assert 28 'int add7(int a, int b, int c, int d, int e, int f, int g) { return a+b+c+d+e+f+g; } int main() { return add7(1,2,3,4,5,6,7); }'

# Step 3.4: forward declarations and void functions
assert 3 'int ret3(); int ret3() { return 3; } int main() { return ret3(); }'
assert 7 'int add(int a, int b); int main() { return add(3, 4); } int add(int a, int b) { return a + b; }'
assert 0 'void noop() { return; } int main() { noop(); return 0; }'
assert 5 'void noop() {} int main() { noop(); return 5; }'

# Step 3.5: block scope and variable shadowing
assert 1 'int main() { int a = 1; { int a = 2; } return a; }'
assert 2 'int main() { int a = 1; { int a = 2; return a; } }'
assert 3 'int main() { int a = 1; { int a = 2; } { int a = 3; return a; } }'
assert 5 'int main() { int a = 1; { int a = 2; { int a = 3; } } return a + 4; }'
assert 3 'int main() { int a = 1; { a = 3; } return a; }'

# Step 3.6: global variables
assert 5 'int g; int main() { g = 5; return g; }'
assert 10 'int x; int y; int main() { x = 3; y = 7; return x + y; }'
assert 2 'int g; void set(int v) { g = v; } int main() { set(2); return g; }'
assert 3 'int g; int main() { int g = 3; return g; }'

# Step 2.15: continue, goto, labels
assert 25 'int main() { int s = 0; int i; for (i = 0; i < 10; i++) { if (i % 2 == 0) continue; s += i; } return s; }'
assert 2 'int main() { goto end; return 1; end: return 2; }'
assert 10 'int main() { int i = 0; loop: i++; if (i < 10) goto loop; return i; }'
assert 55 'int main() { int s = 0; int i; for (i = 1; i <= 10; i++) { s += i; if (i == 10) continue; } return s; }'

# Step 4.2: char type
assert 65 'int main() { char a = 65; return a; }'
assert 3 'int main() { char a = 1; char b = 2; return a + b; }'
assert 65 'char g; int main() { g = 65; return g; }'
assert 97 'int foo(char a) { return a; } int main() { return foo(97); }'
assert 0 'int main() { char a = 0; return a; }'
assert 255 'int main() { char a = 255; return a & 255; }'

# Step 4.3: short and long types
assert 42 'int main() { short a = 42; return a; }'
assert 3 'int main() { short a = 1; short b = 2; return a + b; }'
assert 42 'short g; int main() { g = 42; return g; }'
assert 42 'int main() { long a = 42; return a; }'
assert 3 'int main() { long a = 1; long b = 2; return a + b; }'
assert 42 'long g; int main() { g = 42; return g; }'
assert 6 'int main() { char a = 1; short b = 2; int c = 3; return a + b + c; }'
assert 10 'int add(int a, int b) { return a + b; } int main() { return add(3, 7); }'

# Step 4.4: implicit type conversion
assert 3 'int main() { char a = 1; int b = 2; return a + b; }'
assert 0 'int main() { int a = 256; char b = a; return b; }'
assert 7 'int main() { short a = 3; long b = 4; return a + b; }'
assert 1 'int main() { char a = 1; short b = 2; int c = 3; long d = 4; return a + b + c + d - 9; }'

# Step 4.5: explicit cast
assert 0 'int main() { return (char)256; }'
assert 97 'int main() { return (char)97; }'
assert 1 'int main() { int a = 257; return (char)a; }'
assert 0 'int main() { return (short)65536; }'
assert 42 'int main() { return (int)42; }'
assert 42 'int main() { return (long)42; }'

# Step 4.6: sizeof operator
assert 1 'int main() { return sizeof(char); }'
assert 2 'int main() { return sizeof(short); }'
assert 4 'int main() { return sizeof(int); }'
assert 8 'int main() { return sizeof(long); }'

# Step 4.7: unsigned types
assert 200 'int main() { unsigned char a = 200; return a; }'
assert 200 'int main() { unsigned int a = 200; return a; }'
assert 42 'int main() { unsigned long a = 42; return a; }'
assert 100 'int main() { unsigned short a = 100; return a; }'
assert 42 'int main() { unsigned a = 42; return a; }'
assert 4 'int main() { return sizeof(unsigned int); }'
assert 8 'int main() { return sizeof(unsigned long); }'
assert 0 'int main() { return (unsigned char)256; }'
assert 200 'unsigned char g; int main() { g = 200; return g; }'

# Step 4.8: _Bool type
assert 1 'int main() { _Bool a = 1; return a; }'
assert 0 'int main() { _Bool a = 0; return a; }'
assert 1 'int main() { _Bool a = 42; return a; }'
assert 1 'int main() { _Bool a = 255; return a; }'
assert 1 'int main() { return (_Bool)42; }'
assert 0 'int main() { return (_Bool)0; }'
assert 1 'int main() { return sizeof(_Bool); }'

# Step 5.1: address-of and dereference
assert 3 'int main() { int a = 3; int *p = &a; return *p; }'
assert 10 'int main() { int a = 5; int *p = &a; *p = 10; return a; }'
assert 3 'int main() { int a = 3; return *&a; }'
assert 5 'int main() { int x = 5; int *p = &x; int **pp = &p; return **pp; }'

# Step 5.2: pointer arithmetic
assert 2 'int main() { int a = 1; int b = 2; int *p = &a; p = p - 1; return *p; }'
assert 1 'int main() { int a = 1; int b = 2; int *pa = &a; int *pb = &b; return pa - pb; }'

# Step 5.3: arrays
assert 1 'int main() { int a[3]; *a = 1; return *a; }'
assert 2 'int main() { int a[3]; *(a + 1) = 2; return *(a + 1); }'
assert 3 'int main() { int a[3]; a[0] = 1; a[1] = 2; a[2] = 3; return a[2]; }'
assert 6 'int main() { int a[3]; a[0] = 1; a[1] = 2; a[2] = 3; return a[0] + a[1] + a[2]; }'
assert 5 'int main() { int a[2]; a[0] = 2; a[1] = 3; return a[0] + a[1]; }'
assert 10 'int main() { int a[5]; int i; for (i = 0; i < 5; i++) a[i] = i; int s = 0; for (i = 0; i < 5; i++) s += a[i]; return s; }'
assert 1 'int main() { char a[3]; a[0] = 1; return a[0]; }'
assert 3 'int main() { int a[3]; a[0] = 1; a[1] = 2; a[2] = 3; int *p = a; return p[2]; }'

# Step 5.4: multi-dimensional arrays
assert 42 'int main() { int a[2][3]; a[0][0] = 42; return a[0][0]; }'
assert 6 'int main() { int a[2][3]; a[1][2] = 6; return a[1][2]; }'
assert 15 'int main() { int a[2][3]; int i; int j; int v = 0; for (i = 0; i < 2; i++) for (j = 0; j < 3; j++) { a[i][j] = v; v++; } return a[0][0] + a[0][1] + a[0][2] + a[1][0] + a[1][1] + a[1][2]; }'
assert 5 'int main() { int a[2][3]; a[1][2] = 5; return a[1][2]; }'
assert 7 'int main() { char a[2][3]; a[0][1] = 7; return a[0][1]; }'

# Step 5.5: global arrays
assert 3 'int a[3]; int main() { a[0] = 1; a[1] = 2; a[2] = 3; return a[2]; }'
assert 6 'int a[3]; int main() { a[0] = 1; a[1] = 2; a[2] = 3; return a[0] + a[1] + a[2]; }'
assert 42 'int a[2][3]; int main() { a[1][2] = 42; return a[1][2]; }'
assert 10 'int a[5]; int main() { int i; for (i = 0; i < 5; i++) a[i] = i; int s = 0; for (i = 0; i < 5; i++) s += a[i]; return s; }'

# Step 5.6: sizeof with arrays and expressions
assert 12 'int main() { int a[3]; return sizeof(a); }'
assert 4 'int main() { int a[3]; return sizeof(a[0]); }'
assert 24 'int main() { int a[2][3]; return sizeof(a); }'
assert 12 'int main() { int a[2][3]; return sizeof(a[0]); }'
assert 4 'int main() { int x = 5; return sizeof(x); }'
assert 8 'int main() { int *p; return sizeof(p); }'
assert 3 'int main() { char a[3]; return sizeof(a); }'
assert 1 'int main() { char a[3]; return sizeof(a[0]); }'

# Step 6.1: string literals
assert 104 'int main() { char *s = "hello"; return s[0]; }'
assert 101 'int main() { char *s = "hello"; return s[1]; }'
assert 0 'int main() { char *s = "hello"; return s[5]; }'
assert 97 'int main() { return "abc"[0]; }'
assert 99 'int main() { return "abc"[2]; }'
assert 10 'int main() { return "\n"[0]; }'
assert 0 'int main() { return "\0"[0]; }'

# Step 6.2: full escape sequences
assert 9 'int main() { return "\t"[0]; }'
assert 13 'int main() { return "\r"[0]; }'
assert 7 'int main() { return "\a"[0]; }'
assert 8 'int main() { return "\b"[0]; }'
assert 92 'int main() { return "\\"[0]; }'
assert 65 'int main() { return "\x41"[0]; }'
assert 255 'int main() { return "\xff"[0] & 255; }'
assert 65 'int main() { return "\101"[0]; }'
assert 0 'int main() { return "\0"[0]; }'

# Step 6.3: character literals
assert 97 "int main() { return 'a'; }"
assert 65 "int main() { return 'A'; }"
assert 48 "int main() { return '0'; }"
assert 10 "int main() { return '\\n'; }"
assert 92 "int main() { return '\\\\'; }"
assert 0 "int main() { return '\\0'; }"

# Step 6.4: string concatenation
assert 104 'int main() { char *s = "hel" "lo"; return s[0]; }'
assert 111 'int main() { char *s = "hel" "lo"; return s[4]; }'
assert 0 'int main() { char *s = "hel" "lo"; return s[5]; }'
assert 97 'int main() { char *s = "a" "b" "c"; return s[0]; }'
assert 99 'int main() { char *s = "a" "b" "c"; return s[2]; }'

# Step 7.1: struct definition and member access
assert 3 'int main() { struct { int x; int y; } s; s.x = 1; s.y = 2; return s.x + s.y; }'
assert 10 'int main() { struct { int x; int y; } s; s.x = 10; return s.x; }'
assert 20 'int main() { struct { int x; int y; } s; s.y = 20; return s.y; }'
assert 8 'int main() { return sizeof(struct { int x; int y; }); }'
assert 5 'int main() { struct { int a; int b; int c; } s; s.a = 1; s.b = 2; s.c = 2; return s.a + s.b + s.c; }'
assert 1 'int main() { struct { char a; int b; } s; s.a = 1; return s.a; }'
assert 42 'int main() { struct { char a; int b; } s; s.b = 42; return s.b; }'
assert 8 'int main() { return sizeof(struct { char a; int b; }); }'
assert 3 'int main() { struct { int x; } s; s.x = 3; int *p = &s.x; return *p; }'
assert 7 'int main() { struct { int a; int b; } s; s.a = 3; s.b = 4; int *p = &s.b; return s.a + *p; }'

# Step 7.2: alignment and padding
assert 16 'int main() { return sizeof(struct { char a; long b; }); }'
assert 4 'int main() { return sizeof(struct { char a; char b; short c; }); }'
assert 12 'int main() { return sizeof(struct { char a; int b; char c; }); }'
assert 24 'int main() { return sizeof(struct { char a; long b; char c; }); }'
assert 2 'int main() { return sizeof(struct { char a; char b; }); }'
assert 42 'int main() { struct { char a; long b; } s; s.b = 42; return s.b; }'
assert 3 'int main() { struct { char a; int b; char c; } s; s.a = 1; s.b = 2; s.c = 3; return s.c; }'

# Step 7.3: arrow operator ->
assert 3 'int main() { struct { int x; int y; } s; s.x = 1; s.y = 2; struct { int x; int y; } *p = &s; return p->x + p->y; }'
assert 10 'int main() { struct { int a; } s; s.a = 10; struct { int a; } *p = &s; return p->a; }'
assert 42 'int main() { struct { int x; int y; } s; struct { int x; int y; } *p = &s; p->x = 42; return s.x; }'
assert 7 'int main() { struct { int a; int b; } s; s.a = 3; s.b = 4; struct { int a; int b; } *p = &s; return p->a + p->b; }'

# Step 7.4: tagged structs
assert 3 'int main() { struct Point { int x; int y; }; struct Point p; p.x = 1; p.y = 2; return p.x + p.y; }'
assert 10 'int main() { struct Foo { int a; }; struct Foo f; f.a = 10; return f.a; }'
assert 7 'int main() { struct S { int x; int y; }; struct S a; struct S b; a.x = 3; b.x = 4; return a.x + b.x; }'
assert 42 'int main() { struct S { int val; }; struct S s; s.val = 42; struct S *p = &s; return p->val; }'

# Step 7.5: unions
assert 4 'int main() { return sizeof(union { int a; int b; }); }'
assert 8 'int main() { return sizeof(union { int a; long b; }); }'
assert 4 'int main() { return sizeof(union { char a; int b; }); }'
assert 42 'int main() { union { int a; int b; } u; u.a = 42; return u.b; }'
assert 3 'int main() { union { int x; char y; } u; u.x = 3; return u.y; }'
assert 10 'int main() { union U { int a; int b; }; union U u; u.a = 10; return u.b; }'

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
