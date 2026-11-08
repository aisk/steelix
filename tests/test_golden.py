"""Golden snapshots of the generated Rust module for each case.

`tests/golden/traceback_lines/` has a few cases built with
`--traceback-lines`, which marks the line of each statement that may raise
(design §9).
"""

from __future__ import annotations

from pathlib import Path

import pytest
from conftest import CASES, CASES_DIR, GOLDEN_DIR

from steelix.emit.crate import render_module
from steelix.frontend.parse import parse_file

TRACEBACK_LINES_DIR = GOLDEN_DIR / "traceback_lines"
TRACEBACK_LINES_CASES = [
    "fib",
    "control_flow",
    "try_except",
    "classes_basic",
    "async_with_for",
    "generators",
    "match_basic",
    "match_async",
]


def check_golden(actual: str, golden: Path, update_golden: bool) -> None:
    if update_golden or not golden.exists():
        golden.parent.mkdir(exist_ok=True)
        golden.write_text(actual, encoding="utf-8")
        if not update_golden:
            pytest.fail(f"created missing golden {golden.name}; review and commit it")
        return
    assert actual == golden.read_text(encoding="utf-8"), (
        f"{golden.name} is out of date; rerun with --update-golden if the change is intended"
    )


@pytest.mark.parametrize("case", CASES)
def test_golden(case: str, update_golden: bool) -> None:
    actual = render_module(parse_file(CASES_DIR / f"{case}.py"))
    check_golden(actual, GOLDEN_DIR / f"{case}.rs", update_golden)


@pytest.mark.parametrize("case", TRACEBACK_LINES_CASES)
def test_golden_traceback_lines(case: str, update_golden: bool) -> None:
    actual = render_module(parse_file(CASES_DIR / f"{case}.py"), traceback_lines=True)
    check_golden(actual, TRACEBACK_LINES_DIR / f"{case}.rs", update_golden)
