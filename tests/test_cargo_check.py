"""`cargo check` and `cargo clippy` over the golden snapshots: cheaper than
the e2e tests, and they also assert that the generated code compiles
without warnings and is clippy-clean (with the crate-level allow list of
`steelix.emit.crate.CRATE_ALLOWS`)."""

from __future__ import annotations

import shutil
import subprocess

import pytest
from conftest import CASES, Workspace, compiler_warnings, make_workspace

from steelix.emit.crate import Mode

pytestmark = pytest.mark.cargo


@pytest.fixture(scope="module")
def workspace() -> Workspace:
    return make_workspace("check", CASES, Mode.BIN, from_golden=True)


def test_goldens_compile_without_warnings(workspace: Workspace) -> None:
    result = workspace.cargo("check", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)


def _has_clippy() -> bool:
    if shutil.which("cargo") is None:
        return False
    probe = subprocess.run(["cargo", "clippy", "--version"], capture_output=True, check=False)
    return probe.returncode == 0


@pytest.mark.skipif(not _has_clippy(), reason="clippy is not installed")
def test_goldens_are_clippy_clean(workspace: Workspace) -> None:
    result = workspace.cargo("clippy", "--workspace", "--keep-going", "--message-format=json")
    lints = compiler_warnings(result.stdout, workspace, ("warning", "error"))
    assert not lints, "\n".join(lints)
    assert result.returncode == 0, result.stderr
