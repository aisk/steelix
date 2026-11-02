"""Golden snapshots of the generated Rust module for each case."""

from __future__ import annotations

import pytest
from conftest import CASES, CASES_DIR, GOLDEN_DIR

from steelix.emit.crate import render_module
from steelix.frontend.parse import parse_file


@pytest.mark.parametrize("case", CASES)
def test_golden(case: str, update_golden: bool) -> None:
    actual = render_module(parse_file(CASES_DIR / f"{case}.py"))
    golden = GOLDEN_DIR / f"{case}.rs"
    if update_golden or not golden.exists():
        GOLDEN_DIR.mkdir(exist_ok=True)
        golden.write_text(actual)
        if not update_golden:
            pytest.fail(f"created missing golden {golden.name}; review and commit it")
        return
    assert actual == golden.read_text(), (
        f"{golden.name} is out of date; rerun with --update-golden if the change is intended"
    )
