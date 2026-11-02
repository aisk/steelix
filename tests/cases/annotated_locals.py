"""Annotated assignments in functions: the annotations are never evaluated."""


def describe(items: list) -> str:
    count: int = len(items)
    label: UndefinedType
    total: float = 0
    for item in items:
        total += item
    if count:
        label = "average"
        return label + " " + str(total / count)
    return label


print(describe([1, 2, 3]))
print(describe([]))
