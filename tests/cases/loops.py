"""for/while loops with break, continue and else."""


def find(items, target):
    for item in items:
        if item == target:
            print("found", item)
            break
    else:
        print("not found", target)


def first_even(numbers):
    result = None
    for n in numbers:
        if n % 2:
            continue
        result = n
        break
    return result


def last_item(items):
    for item in items:
        pass
    return item


def count_down(n):
    while n > 0:
        n = n - 1
        if n == 2:
            print("stop at", n)
            break
    else:
        print("ran out")
    return n


def pairs(mapping):
    total = 0
    for key, value in mapping.items():
        print(key, value)
        total = total + value
    return total


def nested(rows):
    for row in rows:
        for cell in row:
            if cell < 0:
                print("negative in", row)
                break
            if cell == 0:
                continue
            print("cell", cell)
        else:
            print("row done")


def search(rows, target):
    i = 0
    for row in rows:
        j = 0
        for cell in row:
            if cell == target:
                return i, j
            j = j + 1
        i = i + 1
    return None


def spin(limit):
    k = 0
    while True:
        k = k + 1
        if k >= limit:
            break
    return k


find([1, 2, 3], 2)
find([1, 2, 3], 5)
print(first_even([1, 3, 4, 6]), first_even([1, 3]))
print(last_item("abc"))
print(count_down(5), count_down(1))
print(pairs({"a": 1, "b": 2}))
nested([[1, 0, 2], [3, -1, 4], []])
print(search([[1, 2], [3, 4]], 4), search([[1]], 9))
print(spin(4))

total = 0
for x in range(5):
    if x == 3:
        continue
    total = total + x
else:
    print("loop finished with x =", x)
print(total)
for word in ["alpha", "beta"]:
    print(word)
    break
else:
    print("unreachable else")
