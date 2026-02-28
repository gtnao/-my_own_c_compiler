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

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
