"""Drives skime, a small Scheme implementation in Python: the `skime`
package next to it is an unmodified copy of https://github.com/aisk/skime
0.3.1 (MIT license, see LICENSE).

    python app.py FILE.scm      feeds FILE.scm to the REPL, one expression a line
    python app.py load FILE.scm loads FILE.scm with the embedding API
    python app.py eval EXPR     evaluates EXPR with the embedding API
"""

import io
import re
import sys

from skime.__main__ import main
from skime.errors import Error
from skime.vm import VM


def repl(path):
    # The REPL shows procedures with their address, which differs between runs.
    out = io.StringIO()
    with open(path) as source:
        sys.stdin, sys.stdout = source, out
        try:
            main()
        finally:
            sys.stdin, sys.stdout = sys.__stdin__, sys.__stdout__
            print(re.sub(r" at 0x[0-9a-fA-F]+", "", out.getvalue()), end="")


def evaluate(how, argument):
    vm = VM()
    try:
        result = vm.load(argument) if how == "load" else vm.eval_string(argument)
    except Error as e:
        print(type(e).__name__, e)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    if sys.argv[1] in ("load", "eval"):
        sys.exit(evaluate(sys.argv[1], sys.argv[2]))
    repl(sys.argv[1])
