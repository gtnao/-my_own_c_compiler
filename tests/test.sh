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

assert_output() {
  expected_output="$1"
  input="$2"

  echo "$input" > "$TMPDIR/tmp.c"
  $COMPILER "$TMPDIR/tmp.c" > "$TMPDIR/tmp.s"
  gcc -o "$TMPDIR/tmp" "$TMPDIR/tmp.s"
  actual_output=$("$TMPDIR/tmp")

  if [ "$actual_output" = "$expected_output" ]; then
    echo "OK: output '$actual_output'"
    PASS=$((PASS + 1))
  else
    echo "FAIL: output '$actual_output' (expected '$expected_output')"
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

# Step 7.6: nested structs/unions
assert 6 'int main() { struct { struct { int x; int y; } inner; int z; } s; s.inner.x = 1; s.inner.y = 2; s.z = 3; return s.inner.x + s.inner.y + s.z; }'
assert 12 'int main() { return sizeof(struct { struct { int x; int y; } inner; int z; }); }'
assert 42 'int main() { struct { union { int a; int b; } u; int c; } s; s.u.a = 42; return s.u.b; }'
assert 5 'int main() { struct O { struct I { int x; } inner; }; struct O o; o.inner.x = 5; return o.inner.x; }'

# Step 8.1: enum
assert 0 'int main() { enum { A, B, C }; return A; }'
assert 1 'int main() { enum { A, B, C }; return B; }'
assert 2 'int main() { enum { A, B, C }; return C; }'
assert 10 'int main() { enum { X = 10, Y, Z }; return X; }'
assert 11 'int main() { enum { X = 10, Y, Z }; return Y; }'
assert 12 'int main() { enum { X = 10, Y, Z }; return Z; }'
assert 5 'int main() { enum { A = 5 }; return A; }'
assert 4 'int main() { return sizeof(enum { A, B }); }'

# Step 8.2: typedef
assert 42 'int main() { typedef int MyInt; MyInt a = 42; return a; }'
assert 3 'int main() { typedef int *IntPtr; int a = 3; IntPtr p = &a; return *p; }'
assert 4 'int main() { typedef int MyInt; return sizeof(MyInt); }'
assert 5 'typedef int MyInt; MyInt add(MyInt a, MyInt b) { return a + b; } int main() { return add(2, 3); }'
assert 3 'int main() { typedef struct { int x; int y; } Point; Point p; p.x = 1; p.y = 2; return p.x + p.y; }'

# Step 9.1: array initializer
assert 1 'int main() { int a[3] = {1, 2, 3}; return a[0]; }'
assert 2 'int main() { int a[3] = {1, 2, 3}; return a[1]; }'
assert 3 'int main() { int a[3] = {1, 2, 3}; return a[2]; }'
assert 6 'int main() { int a[3] = {1, 2, 3}; return a[0] + a[1] + a[2]; }'
assert 10 'int main() { int a[] = {1, 2, 3, 4}; return a[0] + a[1] + a[2] + a[3]; }'
assert 4 'int main() { int a[] = {1, 2, 3, 4}; return sizeof(a) / sizeof(a[0]); }'

# Step 9.2: struct initializer
assert 3 'int main() { struct { int x; int y; } s = {1, 2}; return s.x + s.y; }'
assert 10 'int main() { struct { int a; int b; int c; } s = {1, 2, 7}; return s.a + s.b + s.c; }'
assert 42 'int main() { struct { int x; } s = {42}; return s.x; }'
assert 3 'int main() { struct S { int x; int y; }; struct S s = {1, 2}; return s.x + s.y; }'

# Step 10.1: comments
assert 42 'int main() { return 42; // this is a comment
}'
assert 3 'int main() { /* comment */ return 3; }'
assert 5 'int main() { int a = 5; /* set a */ return a; }'
assert 10 'int main() { int a = 10; // set a
return a; }'

# Step 11.1: printf/libc calls
assert_output 'hello' 'int printf(); int main() { printf("hello"); return 0; }'
assert_output '42' 'int printf(); int main() { printf("%d", 42); return 0; }'
assert_output '3 + 4 = 7' 'int printf(); int main() { printf("%d + %d = %d", 3, 4, 3+4); return 0; }'

# Step 12.1: array parameter
assert 6 'int sum(int a[], int n) { int s = 0; int i; for (i = 0; i < n; i++) s += a[i]; return s; } int main() { int a[3] = {1, 2, 3}; return sum(a, 3); }'
assert 3 'int first(int a[]) { return a[0]; } int main() { int a[3] = {3, 2, 1}; return first(a); }'

# Step 9.3: designated initializers
# Array designated initializer
assert 10 'int main() { int a[5] = {[2] = 10}; return a[2]; }'
assert 0 'int main() { int a[5] = {[2] = 10}; return a[0]; }'
assert 20 'int main() { int a[5] = {1, 2, [3] = 20, 4}; return a[3]; }'
assert 4 'int main() { int a[5] = {1, 2, [3] = 20, 4}; return a[4]; }'
# Struct designated initializer
assert 30 'int main() { struct { int a; int b; int c; } s = {.b = 20, .c = 30}; return s.c; }'
assert 20 'int main() { struct { int a; int b; int c; } s = {.b = 20, .c = 30}; return s.b; }'
assert 5 'int main() { struct { int x; int y; } p = {.x = 5, .y = 10}; return p.x; }'

# Step 9.5: global variable static initialization
assert 42 'int g = 42; int main() { return g; }'
assert 3 'int a = 1; int b = 2; int main() { return a + b; }'
assert 8 'int g[3] = {1, 2, 3}; int main() { return g[0] + g[1] + g[2] + g[0] * g[1]; }'
assert 104 'char s[] = "hello"; int main() { return s[0]; }'
assert 0 'char s[] = "hello"; int main() { return s[5]; }'

# Step 10.4: #define (function-like macros)
assert 7 '#define MAX(a,b) ((a)>(b)?(a):(b))
int main() { return MAX(3, 7); }'
assert 15 '#define ADD(x,y) ((x)+(y))
int main() { return ADD(7, 8); }'
assert 9 '#define SQ(x) ((x)*(x))
int main() { return SQ(3); }'

# Step 10.3: #define (object-like macros)
assert 42 '#define N 42
int main() { return N; }'
assert 10 '#define X 3
#define Y 7
int main() { return X + Y; }'
assert 5 '#define VAL 5
int main() { int a = VAL; return a; }'

# Step 11.2: variadic arguments
assert 60 'int sum(int n, ...) { va_list ap; va_start(ap, n); int total = 0; int i; for (i = 0; i < n; i++) total += va_arg(ap, int); va_end(ap); return total; } int main() { return sum(3, 10, 20, 30); }'
assert 6 'int sum(int n, ...) { va_list ap; va_start(ap, n); int total = 0; int i; for (i = 0; i < n; i++) total += va_arg(ap, int); va_end(ap); return total; } int main() { return sum(3, 1, 2, 3); }'

# Step 10.8: #error, #warning, #line, #pragma
assert 42 '#pragma once
int main() { return 42; }'
assert 10 '#warning testing
int main() { return 10; }'

# Step 10.7: predefined macros (__FILE__, __LINE__)
assert 1 'int main() { return __LINE__; }'
assert 2 'int x;
int main() { return __LINE__; }'

# Step 10.6: # (stringize) and ## (token paste) operators
assert_output 'hello' '#define STR(x) #x
int printf();
int main() { printf(STR(hello)); return 0; }'
assert 12 '#define CONCAT(a,b) a##b
int main() { int xy = 12; return CONCAT(x,y); }'
assert 42 '#define VAR(n) var##n
int main() { int var1 = 42; return VAR(1); }'

# Step 10.5: conditional compilation (#ifdef, #ifndef, #if, #else, #elif, #endif)
assert 1 '#define FOO
#ifdef FOO
int main() { return 1; }
#else
int main() { return 2; }
#endif'
assert 2 '#ifdef FOO
int main() { return 1; }
#else
int main() { return 2; }
#endif'
assert 10 '#define X 10
#ifndef X
int main() { return 0; }
#else
int main() { return X; }
#endif'
assert 5 '#ifndef UNDEF
int main() { return 5; }
#else
int main() { return 0; }
#endif'
assert 1 '#if 1
int main() { return 1; }
#else
int main() { return 0; }
#endif'
assert 0 '#if 0
int main() { return 1; }
#else
int main() { return 0; }
#endif'
assert 20 '#define X 2
#if X == 1
int main() { return 10; }
#elif X == 2
int main() { return 20; }
#else
int main() { return 30; }
#endif'

# Step 10.2: #include
# Create test header file
echo 'int add(int a, int b) { return a + b; }' > "$TMPDIR/add.h"
echo '#include "add.h"
int main() { return add(3, 4); }' > "$TMPDIR/include_test.c"
$COMPILER "$TMPDIR/include_test.c" > "$TMPDIR/include_test.s"
gcc -o "$TMPDIR/include_test" "$TMPDIR/include_test.s"
actual=$("$TMPDIR/include_test"; echo $?)
if [ "$actual" = "7" ]; then
  echo "OK: include test => 7"
  PASS=$((PASS + 1))
else
  echo "FAIL: include test => $actual (expected 7)"
  FAIL=$((FAIL + 1))
fi

# Step 9.7: extern declaration
assert 5 'extern int g; int g = 5; int main() { return g; }'
assert 0 'extern int printf(); int main() { printf("hello"); return 0; }'

# Step 9.6: static local variables
assert 3 'int count() { static int c = 0; c++; return c; } int main() { count(); count(); return count(); }'
assert 10 'int add(int x) { static int sum = 0; sum += x; return sum; } int main() { add(1); add(2); add(3); return add(4); }'

# Step 9.4: compound literals
assert 3 'int main() { int *p = (int[]){1, 2, 3}; return p[2]; }'
assert 1 'int main() { int *p = (int[]){1, 2, 3}; return p[0]; }'
assert 6 'int main() { int *p = (int[3]){1, 2, 3}; return p[0] + p[1] + p[2]; }'
assert 10 'int main() { return ((struct { int a; int b; }){3, 7}).a + ((struct { int a; int b; }){3, 7}).b; }'

# Step 12.3: string initialization for char arrays
assert 104 'int main() { char s[] = "hello"; return s[0]; }'
assert 111 'int main() { char s[] = "hello"; return s[4]; }'
assert 0 'int main() { char s[] = "hello"; return s[5]; }'
assert 6 'int main() { char s[] = "hello"; return sizeof(s); }'

# Step 13.7: comprehensive tests
# FizzBuzz (count FizzBuzz outputs)
assert 65 'int main() { int count = 0; int i; for (i = 1; i <= 100; i++) { if (i % 15 == 0) count += 4; else if (i % 3 == 0) count += 1; else if (i % 5 == 0) count += 1; } return count; }'

# Fibonacci
assert 55 'int fib(int n) { if (n <= 1) return n; return fib(n-1) + fib(n-2); } int main() { return fib(10); }'
assert 89 'int fib(int n) { if (n <= 1) return n; return fib(n-1) + fib(n-2); } int main() { return fib(11); }'

# Iterative factorial
assert 120 'int main() { int n = 5; int result = 1; int i; for (i = 2; i <= n; i++) result *= i; return result; }'

# Bubble sort
assert 1 'int main() { int a[] = {5, 3, 1, 4, 2}; int i; int j; for (i = 0; i < 5; i++) for (j = 0; j < 4; j++) if (a[j] > a[j+1]) { int t = a[j]; a[j] = a[j+1]; a[j+1] = t; } return a[0]; }'
assert 5 'int main() { int a[] = {5, 3, 1, 4, 2}; int i; int j; for (i = 0; i < 5; i++) for (j = 0; j < 4; j++) if (a[j] > a[j+1]) { int t = a[j]; a[j] = a[j+1]; a[j+1] = t; } return a[4]; }'

# GCD (Euclidean algorithm)
assert 6 'int gcd(int a, int b) { while (b != 0) { int t = b; b = a % b; a = t; } return a; } int main() { return gcd(48, 18); }'
assert 1 'int gcd(int a, int b) { while (b != 0) { int t = b; b = a % b; a = t; } return a; } int main() { return gcd(7, 13); }'

# Power function
assert 8 'int power(int base, int exp) { int result = 1; while (exp > 0) { result *= base; exp--; } return result; } int main() { return power(2, 3); }'
assert 27 'int power(int base, int exp) { int result = 1; while (exp > 0) { result *= base; exp--; } return result; } int main() { return power(3, 3); }'

# Step 11.4: Callback pattern
assert 10 'int apply(int (*f)(int), int x) { return f(x); } int dbl(int x) { return x * 2; } int main() { return apply(dbl, 5); }'
assert 25 'int apply(int (*f)(int), int x) { return f(x); } int sq(int x) { return x * x; } int main() { return apply(sq, 5); }'
assert 12 'int map_sum(int *a, int n, int (*f)(int)) { int s = 0; int i; for (i = 0; i < n; i++) s += f(a[i]); return s; } int dbl(int x) { return x * 2; } int main() { int a[3] = {1, 2, 3}; return map_sum(a, 3, dbl); }'

# Step 12.6: for loop scope
assert 100 'int main() { int i = 100; for (int i = 0; i < 5; i++) {} return i; }'
assert 45 'int main() { int s = 0; for (int i = 0; i < 10; i++) s += i; return s; }'

# Step 12.8: Multiple variable declarations
assert 3 'int main() { int a = 1, b = 2; return a + b; }'
assert 6 'int main() { int a = 1, b = 2, c = 3; return a + b + c; }'
assert 3 'int main() { int a, b; a = 1; b = 2; return a + b; }'

# Step 12.4-12.5: const and volatile qualifiers
assert 42 'int main() { const int a = 42; return a; }'
assert 3 'int main() { const int *p; int a = 3; p = &a; return *p; }'
assert 5 'int main() { volatile int a = 5; return a; }'

# Step 13.7: Comprehensive tests

# Struct-based linked list simulation (via array)
assert 15 'int main() { int vals[5] = {1, 2, 3, 4, 5}; int sum = 0; int i; for (i = 0; i < 5; i++) sum += vals[i]; return sum; }'

# Sieve of Eratosthenes (count primes up to 30)
assert 10 'int main() { int sieve[31]; int i, j; for (i = 0; i <= 30; i++) sieve[i] = 1; sieve[0] = 0; sieve[1] = 0; for (i = 2; i * i <= 30; i++) if (sieve[i]) for (j = i * i; j <= 30; j += i) sieve[j] = 0; int count = 0; for (i = 2; i <= 30; i++) if (sieve[i]) count++; return count; }'

# Matrix multiplication (2x2)
assert 19 'int main() { int a[4] = {1, 2, 3, 4}; int b[4] = {5, 6, 7, 8}; int c[4]; c[0] = a[0]*b[0] + a[1]*b[2]; c[1] = a[0]*b[1] + a[1]*b[3]; c[2] = a[2]*b[0] + a[3]*b[2]; c[3] = a[2]*b[1] + a[3]*b[3]; return c[0]; }'

# Nested function calls with recursion (Ackermann A(2,3))
assert 9 'int ack(int m, int n) { if (m == 0) return n + 1; if (n == 0) return ack(m - 1, 1); return ack(m - 1, ack(m, n - 1)); } int main() { return ack(2, 3); }'

# Function pointer dispatch
assert 15 'int dbl(int x) { return x * 2; } int triple(int x) { return x * 3; } int main() { int (*f)(int) = dbl; int a = f(3); f = triple; int b = f(3); return a + b; }'

# Step 11.3: Function pointers
assert 7 'int add(int a, int b) { return a + b; } int main() { int (*fp)(int, int) = add; return fp(3, 4); }'
assert 2 'int sub(int a, int b) { return a - b; } int main() { int (*fp)(int, int) = sub; return fp(5, 3); }'
assert 42 'int ret42() { return 42; } int main() { int (*fp)() = ret42; return fp(); }'

# Step 12.7: Complex type declarations
# Pointer to array: int (*p)[N]
assert 20 'int main() { int a[3] = {10, 20, 30}; int (*p)[3] = &a; return (*p)[1]; }'
# Array of pointers: int *a[N]
assert 6 'int main() { int a=1; int b=2; int c=3; int *arr[3]; arr[0]=&a; arr[1]=&b; arr[2]=&c; return *arr[0] + *arr[1] + *arr[2]; }'

# Step 12.9: Bit-fields
assert 5 'int main() { struct { int a : 4; int b : 4; } s; s.a = 5; s.b = 3; return s.a; }'
assert 3 'int main() { struct { int a : 4; int b : 4; } s; s.a = 5; s.b = 3; return s.b; }'
assert 7 'int main() { struct { int x : 3; int y : 5; } s; s.x = 7; s.y = 31; return s.x; }'
assert 31 'int main() { struct { int x : 3; int y : 5; } s; s.x = 7; s.y = 31; return s.y; }'

# Step 12.12: _Generic
assert 1 'int main() { int x = 0; return _Generic(x, int: 1, long: 2, default: 0); }'
assert 2 'int main() { long x = 0; return _Generic(x, int: 1, long: 2, default: 0); }'
assert 0 'int main() { char x = 0; return _Generic(x, int: 1, long: 2, default: 0); }'

# Step 12.11: _Alignof and _Alignas
assert 4 'int main() { return _Alignof(int); }'
assert 8 'int main() { return _Alignof(long); }'
assert 1 'int main() { return _Alignof(char); }'
assert 8 'int main() { return _Alignof(int *); }'
assert 5 'int main() { _Alignas(16) int x = 5; return x; }'

# Step 12.10: Flexible array member and struct array members
assert 3 'int main() { struct { int x; int arr[3]; } s; s.arr[0]=1; s.arr[1]=2; s.arr[2]=3; return s.arr[1]+s.arr[0]; }'
assert 4 'int main() { struct { int len; int data[]; } *p; char buf[20]; p = (struct { int len; int data[]; } *)buf; p->len = 3; p->data[0] = 4; return p->data[0]; }'

# Step 14.1: long long and short int type specifiers
assert 42 'int main() { long long a = 42; return a; }'
assert 3 'int main() { long long int a = 1; long long int b = 2; return a + b; }'
assert 8 'int main() { return sizeof(long long); }'
assert 8 'int main() { return sizeof(long long int); }'
assert 42 'int main() { unsigned long long a = 42; return a; }'
assert 8 'int main() { return sizeof(unsigned long long); }'
assert 42 'int main() { long int a = 42; return a; }'
assert 8 'int main() { return sizeof(long int); }'
assert 42 'int main() { short int a = 42; return a; }'
assert 2 'int main() { return sizeof(short int); }'
assert 100 'int main() { unsigned short int a = 100; return a; }'

# Step 12.2: Struct value copy and pass/return
assert 3 'int main() { struct { int x; int y; } s1; s1.x = 1; s1.y = 2; struct { int x; int y; } s2; s2.x = 0; s2.y = 0; s2 = s1; return s2.x + s2.y; }'
assert 10 'struct P { int x; int y; }; int main() { struct P a; a.x = 3; a.y = 7; struct P b; b = a; return b.x + b.y; }'
# Struct return from function
assert 3 'struct P { int x; int y; }; struct P make() { struct P p; p.x = 1; p.y = 2; return p; } int main() { struct P r = make(); return r.x + r.y; }'
# Struct pass by value (original not modified)
assert 7 'struct P { int x; int y; }; int sum(struct P p) { return p.x + p.y; } int main() { struct P a; a.x = 3; a.y = 4; return sum(a); }'
assert 3 'struct P { int x; int y; }; void modify(struct P p) { p.x = 99; } int main() { struct P a; a.x = 3; a.y = 4; modify(a); return a.x; }'
# Struct with char member
assert 97 'struct S { char c; int n; }; int get(struct S s) { return s.c; } int main() { struct S s; s.c = 97; s.n = 42; return get(s); }'

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
