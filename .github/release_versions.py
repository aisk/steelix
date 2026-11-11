"""Check that the versions agree before a release (docs/design.md §14).

Called by `.github/workflows/release.yml` with the tag (`v0.3.1`). Prints
the crate versions as `name=version` lines for `$GITHUB_OUTPUT`, or exits
with a message when

- the tag is not the version of `steelix` in `pyproject.toml`
- `steelix-runtime` has another version than `steelix`
- `RUNTIME_VERSION`, the dependency that generated crates declare, is not
  the major and minor version of the runtime
- the runtime requires a newer `steelix-runtime-macros` than the one in
  this repository
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def version_of(manifest: str, table: str) -> str:
    with (ROOT / manifest).open("rb") as f:
        return tomllib.load(f)[table]["version"]


def numbers(version: str) -> tuple[int, ...]:
    return tuple(int(part) for part in version.split("."))


def main(tag: str) -> str | None:
    steelix = version_of("pyproject.toml", "project")
    runtime = version_of("crates/steelix-runtime/Cargo.toml", "package")
    macros = version_of("crates/steelix-runtime-macros/Cargo.toml", "package")
    if tag != f"v{steelix}":
        return f"the tag is {tag}, but pyproject.toml has version {steelix}"
    if runtime != steelix:
        return f"steelix is {steelix}, but steelix-runtime is {runtime}"

    crate_py = (ROOT / "src/steelix/emit/crate.py").read_text(encoding="utf-8")
    declared = re.search(r'^RUNTIME_VERSION = "([^"]+)"$', crate_py, re.MULTILINE)
    if declared is None:
        return "no RUNTIME_VERSION in src/steelix/emit/crate.py"
    if numbers(declared[1]) != numbers(runtime)[:2]:
        return f'RUNTIME_VERSION is "{declared[1]}", but steelix-runtime is {runtime}'

    with (ROOT / "crates/steelix-runtime/Cargo.toml").open("rb") as f:
        required = tomllib.load(f)["dependencies"]["steelix-runtime-macros"]["version"]
    if numbers(required) > numbers(macros):
        return f"steelix-runtime requires macros {required}, but they are {macros}"

    print(f"steelix={steelix}")
    print(f"runtime={runtime}")
    print(f"macros={macros}")
    return None


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
