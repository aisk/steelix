# Module level `if`, `for` and `while` statements stay native when only some
# statements in their bodies are kept as Python (design §13).
import sys


def double(x):
    return x * 2


if len(sys.argv) > 5:
    mode = "many"
    labels = list(c for c in "many" for _ in "1")  # kept as Python, the `if` is native

    def describe(n):
        return f"{n} (many)"
else:
    mode = "few"

    def describe(n):
        return f"{n} ({mode})"

    print("native branch", mode)

for i in range(4):
    if i == 1:
        continue
    print(describe(double(i)))
    squares = list(j for j in range(i) for _ in "1")  # kept as Python
    print("squares", squares)

# `break` in a kept statement: the loop is kept whole.
n = 0
while True:
    n += 1
    try:
        if n == 3:
            break
    except* ValueError:
        pass
    print("tried", n)

# A kept loop inside a native one: its `break` is its own.
for word in ["ab", "cde"]:
    for ch in word:
        try:
            if ch == "d":
                break
        except* ValueError:
            pass
        print("char", ch)
    else:
        print("no d in", word)


class Point:
    def __init__(self, x):
        self.x = x


if __name__ == "__main__":
    print("main", mode)

    class Local:
        label = "local class"

    print(Local.label, Point(double(3)).x)
    total = sum(j for j in range(5) for _ in "1")
    print("total", total)
