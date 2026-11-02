def positive(x):
    assert x > 0, f"expected a positive number, got {x}"
    return x


print(positive(1))
print(positive(-1))
