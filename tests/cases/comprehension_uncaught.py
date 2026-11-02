"""An uncaught exception raised inside a generator expression that a list
comprehension consumes: the traceback has the comprehension's line in the
function and the `<genexpr>` frame."""


def totals(rows):
    return [sum(10 // c for c in row) for row in rows]


print(totals([[1, 2]]))
print(totals([[1], [0]]))
