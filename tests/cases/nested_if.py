# A nested `if` stays nested, as in the source (clippy's `collapsible_if` is
# allowed at the crate root).


def check(keep, value):
    if keep:
        if value is not None:
            print("keep", value)
    if isinstance(value, tuple):
        if 1 in value and True not in value:
            return False
    return True


print(check(True, 1), check(False, 2), check(True, None))
print(check(True, (1, 2)), check(True, (True,)), check(False, (0,)))
