"""A program with a package for `steelix build` (tests/test_dist.py): data
files of the package through `importlib.resources` and `pkgutil`, and
`python -m pkg.cli` run by the bundled interpreter."""

import importlib.resources
import os
import pkgutil
import subprocess
import sys

import pkg
from pkg import cli


def main(args):
    print("argv", args)
    if args:
        return
    print(importlib.resources.files("pkg").joinpath("data.txt").read_text().strip())
    template = importlib.resources.files(pkg) / "templates" / "greeting.txt"
    print(template.read_text().strip().format(name=pkg.NAME))
    print(pkgutil.get_data("pkg", "data.txt").decode().strip())
    print(importlib.resources.files(cli).joinpath("data.txt").is_file())
    cli.main(["in-process"])
    here = os.path.dirname(__file__)
    child = subprocess.run(
        [sys.executable, "-m", "pkg.cli", "child"], cwd=here, capture_output=True, text=True
    )
    print(child.returncode, child.stdout.strip(), child.stderr.strip())


if __name__ == "__main__":
    main(sys.argv[1:])
