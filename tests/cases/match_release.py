"""When the objects a `match` takes apart are released (design §12): the
known differences are listed in `KNOWN_DIFFERENCES` in tests/test_e2e.py."""

import collections.abc

alive = set()


class T:
    def __init__(self, name):
        self.name = name
        alive.add(name)

    def __del__(self):
        alive.discard(self.name)


class Fresh:
    """A sequence whose items are new objects, referenced by nothing else."""

    __match_args__ = ("item",)

    def __init__(self, *names):
        self.names = names
        alive.add("subject")

    def __del__(self):
        alive.discard("subject")

    def __len__(self):
        return len(self.names)

    def __getitem__(self, index):
        return T(self.names[index])

    @property
    def item(self):
        return T("attribute")


collections.abc.Sequence.register(Fresh)


def captures():
    match Fresh("a"):
        case [x]:
            print("matched", x.name)
    # A capture only read in its case is released at the end of the case.
    print("captures after the match:", sorted(alive))


def unused():
    match Fresh("b"):
        case [y]:
            print("unused capture:", sorted(alive))


def subject():
    match Fresh("c"):
        case Fresh(T()):
            # CPython has released the subject and the attribute here.
            print("in the body:", sorted(alive))


captures()
unused()
subject()
print("at the end:", sorted(alive))
