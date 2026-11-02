"""`break`/`continue` in a loop's `else` belong to the enclosing loop."""


def skip_rows(rows):
    for row in rows:
        for cell in row:
            if cell < 0:
                break
        else:
            continue
        print("row with a negative cell", row)


def first_complete(rows):
    for row in rows:
        for cell in row:
            if cell is None:
                break
        else:
            print("complete", row)
            break
    else:
        print("none complete")


def countdown(n):
    while n > 0:
        n -= 1
        i = 0
        while i < n:
            if i == 2:
                break
            i += 1
        else:
            print("short", n)
            continue
        print("long", n)


def nested(groups):
    for group in groups:
        for row in group:
            for cell in row:
                if cell == 0:
                    break
            else:
                continue
            print("zero in", row)
            break
        else:
            print("no zero in", group)
            continue
        print("done with", group)


skip_rows([[1, 2], [3, -1], [4]])
first_complete([[1, None], [2, 3], [4]])
first_complete([[None]])
countdown(5)
nested([[[1], [0, 1]], [[2]]])
