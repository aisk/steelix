"""Lambdas in the headers of `for`, `while`, `if` and `elif`."""

totals = {"b": 3, "a": 5, "c": 1}


def report(data):
    for name in sorted(data, key=lambda k: data[k]):
        print(name, data[name])
    n = 0
    while (lambda: n)() < 2:
        n += 1
    if (lambda x: x > 3)(data["a"]):
        print("big a")
    elif (lambda x: x)(0):
        print("unreachable")
    if (lambda: False)():
        print("unreachable")
    elif (lambda x: x < 3)(data["c"]):
        print("small c")


for name in sorted(totals, key=lambda k: -totals[k]):
    print(name)
report(totals)
