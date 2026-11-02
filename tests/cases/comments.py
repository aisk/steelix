#!/usr/bin/env python3
"""Python comments are kept in the generated Rust code."""

# A module level comment before a function.


def total(items):
    # Sum the items one by one.
    result = 0
    for item in items:
        # Skip negative numbers.
        if item < 0:
            continue
        result += item  # trailing comments stay with their statement
    return result


# Comment before a call.
print(total([1, -2, 3]))
# Comment at the end of the module.
