"""Loops and branches over module level state."""

LIMIT = 20
STEP = 3


def collatz_steps(n):
    """Count the steps until the Collatz sequence reaches 1."""
    steps = 0
    while n != 1:
        if n % 2 == 0:
            n = n // 2
        else:
            n = 3 * n + 1
        steps = steps + 1
    return steps


def classify(n):
    if n < 0:
        kind = "negative"
    elif n == 0:
        kind = "zero"
    elif n < LIMIT:
        kind = "small"
    else:
        kind = "large"
    return kind


def count_up(start):
    i = start
    total = 0
    while i < LIMIT:
        total = total + i
        i = i + STEP
    return total


def first_square_above(bound):
    k = 0
    while True:
        k = k + 1
        if k * k > bound:
            return k


print(collatz_steps(27))
print(classify(-5), classify(0), classify(7), classify(100))
print(count_up(1), count_up(LIMIT))
print(first_square_above(50))
n = 10
while n > 0:
    if n % 4 == 0:
        print("four", n)
    elif n % 3 == 0:
        print("three", n)
    n = n - STEP
print(n)
