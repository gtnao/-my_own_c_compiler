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

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
