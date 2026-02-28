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

# Step 1.1: single integer literal
assert 0 '0'
assert 42 '42'
assert 255 '255'
assert 1 '1'
assert 100 '100'

# Step 1.2: addition and subtraction
assert 21 '5+20-4'
assert 0 '0+0'
assert 10 '10'
assert 3 '1+2'
assert 5 '10-5'
assert 15 '1+2+3+4+5'

# Step 1.3: whitespace handling
assert 41 ' 12 + 34 - 5 '
assert 21 ' 5 + 20 - 4 '
assert 10 '  10  '

# Step 1.4: multiplication, division, parentheses, unary operators
assert 47 '5+6*7'
assert 15 '5*(9-6)'
assert 4 '(3+5)/2'
assert 10 '-10+20'
assert 10 '- -10'
assert 10 '- - +10'
assert 6 '2*3'
assert 3 '9/3'
assert 20 '(2+3)*(5-1)'
assert 3 '10/2/2+1-1+1'

# Step 1.5: modulo operator
assert 1 '10 % 3'
assert 0 '6 % 3'
assert 2 '11 % 3'
assert 0 '10 % 5'
assert 1 '7 % 2'

# Step 1.6: comparison operators
assert 1 '1==1'
assert 0 '1==2'
assert 1 '1!=2'
assert 0 '1!=1'
assert 1 '1<2'
assert 0 '2<1'
assert 0 '1<1'
assert 1 '1<=1'
assert 1 '1<=2'
assert 0 '2<=1'
assert 1 '2>1'
assert 0 '1>2'
assert 1 '2>=2'
assert 1 '3>=2'
assert 0 '1>=2'
assert 1 '5+1==6'
assert 1 '3*2>=5'

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
