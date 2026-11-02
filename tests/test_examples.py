"""The example applications in `examples/` (design §15).

`examples/logsum` holds the transpiled module as steelix generates it and a
copy in which a person rewrote some functions in plain Rust. Both must behave
exactly like the Python program.
"""

from __future__ import annotations

import shutil
from pathlib import Path

import pytest
from conftest import (
    E2E_DIR,
    PYTHON,
    ROOT,
    RUNTIME,
    Workspace,
    cargo_env,
    compiler_warnings,
)
from test_e2e import run

from steelix.emit.crate import DEV_PROFILE, CrateOptions, Mode, emit_crate, render_module
from steelix.frontend.parse import parse_file

EXAMPLE = ROOT / "examples" / "logsum"
VARIANTS = ["generated", "rewritten"]
ARGS = [
    ["access.log"],
    ["access.log", "--top", "3", "--slow", "100"],
    ["missing.log"],
    [],
    ["--help"],
]


def test_generated_is_the_transpiler_output(update_golden: bool) -> None:
    """`generated/` is untouched: regenerating it gives the same file.

    `--update-golden` rewrites it. `rewritten/` is maintained by hand.
    """
    actual = render_module(parse_file(EXAMPLE / "logsum.py"))
    generated = EXAMPLE / "generated" / "src" / "logsum.rs"
    if update_golden:
        generated.write_text(actual)
        return
    assert actual == generated.read_text(), (
        "examples/logsum/generated is out of date; rerun with --update-golden if the change"
        " is intended, and check that examples/logsum/rewritten still builds"
    )


@pytest.fixture(scope="session")
def example_workspace() -> Workspace:
    """One crate per variant: the crate steelix emits, with `src/` replaced by
    the variant's sources."""
    root = E2E_DIR / "examples"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    options = CrateOptions(mode=Mode.BIN, runtime_path=RUNTIME, standalone=False)
    parsed = parse_file(EXAMPLE / "logsum.py")
    members = []
    for variant in VARIANTS:
        crate = root / variant
        emit_crate(parsed, crate, options)
        shutil.copytree(EXAMPLE / variant / "src", crate / "src", dirs_exist_ok=True)
        manifest = crate / "Cargo.toml"
        text = manifest.read_text().replace('name = "logsum"', f'name = "logsum-{variant}"')
        manifest.write_text(text)
        members.append(f'"{variant}"')
    manifest = f'[workspace]\nresolver = "3"\nmembers = [{", ".join(members)}]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest)
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    workspace = Workspace(root, VARIANTS)
    result = workspace.cargo("build", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
    return workspace


# Numbers that Python's `int()` accepts and the rewritten `parse_line`, which
# uses `str::parse::<u64>`, does not: a negative size, digits grouped with an
# underscore and non-ASCII digits. This is a deliberate difference, documented
# in the README: the rewritten program counts these lines as malformed.
REWRITE_REJECTS = [" -5 ", " 1_024 ", " \u0664\u0662ms"]


def rewritten_log(tmp_path: Path) -> Path:
    """A copy of the example directory whose `access.log` has the lines that
    the rewritten program rejects replaced by malformed ones, so that CPython
    prints what the rewritten program is expected to print for the original."""
    shutil.copyfile(EXAMPLE / "logsum.py", tmp_path / "logsum.py")
    lines = (EXAMPLE / "access.log").read_text(encoding="utf-8").splitlines(keepends=True)
    changed = [
        "### rejected by the rewritten parse_line ###\n"
        if any(token in line for token in REWRITE_REJECTS)
        else line
        for line in lines
    ]
    assert len([a for a, b in zip(lines, changed, strict=True) if a != b]) == len(REWRITE_REJECTS)
    (tmp_path / "access.log").write_text("".join(changed), encoding="utf-8")
    return tmp_path


@pytest.mark.e2e
@pytest.mark.cargo
@pytest.mark.parametrize("args", ARGS, ids=lambda args: " ".join(args) or "no-args")
@pytest.mark.parametrize("variant", VARIANTS)
def test_example_matches_python(
    variant: str, args: list[str], example_workspace: Workspace, tmp_path: Path
) -> None:
    expected = run([str(PYTHON), "logsum.py", *args], EXAMPLE)
    binary = cargo_env(extension=False)["CARGO_TARGET_DIR"]
    actual = run([str(Path(binary) / "debug" / f"logsum-{variant}"), *args], EXAMPLE)
    if variant == "rewritten" and "access.log" in args:
        # The documented difference: not CPython's output for this log, but
        # CPython's output for the log with those lines made malformed.
        assert actual != expected
        expected = run([str(PYTHON), "logsum.py", *args], rewritten_log(tmp_path))
    assert actual == expected


@pytest.mark.e2e
@pytest.mark.cargo
def test_rewritten_unit_tests_pass(example_workspace: Workspace) -> None:
    """The rewritten functions are plain Rust and have their own unit tests."""
    result = example_workspace.cargo("test", "-p", "logsum-rewritten")
    assert result.returncode == 0, result.stderr + result.stdout
