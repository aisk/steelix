"""Roman numeral and base conversion utility with round-trip checks."""

import string

ROMAN = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
]
DIGITS = string.digits + string.ascii_lowercase


def to_roman(n):
    assert 0 < n < 4000, "out of range"
    out = []
    for value, sym in ROMAN:
        while n >= value:
            out.append(sym)
            n -= value
    return "".join(out)


def from_roman(s):
    values = {}
    for value, sym in ROMAN:
        if len(sym) == 1:
            values[sym] = value
    total = 0
    for i, ch in enumerate(s):
        v = values[ch]
        if i + 1 < len(s) and values[s[i + 1]] > v:
            total -= v
        else:
            total += v
    return total


def to_base(n, base=2):
    if n == 0:
        return "0"
    neg = n < 0
    n = abs(n)
    digits = []
    while n:
        n, r = divmod(n, base)
        digits.append(DIGITS[r])
    if neg:
        digits.append("-")
    return "".join(reversed(digits))


def from_base(s, base):
    return int(s, base)


def main():
    samples = [1, 4, 9, 14, 40, 90, 400, 1994, 2024, 3999]
    for n in samples:
        r = to_roman(n)
        back = from_roman(r)
        assert back == n
        print(f"{n:>5} = {r:<12} hex={n:#x} oct={n:o} bin={n:b}")
    print()
    for n in (0, 255, -42, 123456789):
        row = []
        for base in (2, 7, 16, 36):
            text = to_base(n, base=base)
            assert from_base(text, base) == n
            row.append(f"b{base}:{text}")
        print(f"{n:>10}  " + "  ".join(row))
    for s in ("MMXXIV", "XLII", "CDXLIV"):
        print(s, "->", from_roman(s))
    for n in range(1, 50):
        r = to_roman(n)
        if len(r) >= 7:
            print("first 7-char numeral:", n, r)
            break
    else:
        print("none found")
    print(f"{3999:,} {0.125:.1%} {255:08b} {1e6:.2e}")


if __name__ == "__main__":
    main()
