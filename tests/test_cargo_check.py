"""`cargo check` over the golden snapshots: cheaper than the e2e tests and also
asserts that the generated code compiles without warnings."""

from __future__ import annotations

import pytest
from conftest import CASES, compiler_warnings, make_workspace

from steelix.emit.crate import Mode

pytestmark = pytest.mark.cargo


def test_goldens_compile_without_warnings() -> None:
    workspace = make_workspace("check", CASES, Mode.BIN, from_golden=True)
    result = workspace.cargo("check", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
