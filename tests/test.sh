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

echo ""
echo "--- Results ---"
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
