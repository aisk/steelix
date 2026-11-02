def describe():
    print(0, 7, -7, 2147483648, -9223372036854775808, 2**64, 123456789012345678901234567890123456789012)
    print(1.5, -0.0, 1e100, 2.5e-10, 1e999)
    print("plain", 'quote " and \\ backslash', "tab\tnewline\n", "unicode é 中文 ​")
    print(b"bytes\x00\xff", True, False, None)
    print(7 / 2, 7 // 2, -7 % 3, 2**10, 1 << 5, 255 >> 2, 6 & 3, 6 | 3, 6 ^ 3)
    print("ab" + "cd", "ab" * 3, "%d items" % 5)
    print(3 < 4, 3 <= 2, 3 > 4, 3 >= 3, 3 == 3.0, "a" != "b")
    print(1 + 2.5, 10 - True)
    print(len("a\ud800b"), len(b""), abs(-3))


x = 1 + 2
y = z = x * 2
print(x, y, z)
print(describe())
