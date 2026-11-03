"""Docstrings of several lines are printed one line per source line in the
Rust literal, joined by `\\` continuations, and keep their exact value.

   Indented line.
"""


class K:
    """Class doc.

    More.
    """

    def method(self, a, b=1):
        """Method doc.

            code block
        trailing"""
        return a


def f(x):
    """Summary line.

    Details here with "quotes" and \\ backslash\ttab.
      indented two
    """
    return x


def g():
    """One line only."""


def h():
    """Tabs\tinside and a tab\t
    \tstarting a line, trailing spaces \x20
    ünïcødé, a lone \\ and an escaped \" quote.

    \f form feed\v vertical tab
    """


def k():
    """Ends with blank lines."""


print(repr(__doc__))
print(repr(K.__doc__))
print(repr(K.method.__doc__))
print(repr(f.__doc__))
print(repr(g.__doc__))
print(repr(h.__doc__))
print(repr(k.__doc__))
