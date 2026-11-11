"""Shared fixtures: golden updating and building generated crates with cargo."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import pytest

from steelix.emit.crate import DEV_PROFILE, CrateOptions, Mode, emit_crate, rust_module_name
from steelix.frontend.parse import parse_file

ROOT = Path(__file__).resolve().parents[1]
CASES_DIR = ROOT / "tests" / "cases"
CORPUS_DIR = ROOT / "tests" / "corpus"
APPS_DIR = ROOT / "tests" / "apps"
GOLDEN_DIR = ROOT / "tests" / "golden"
RUNTIME = ROOT / "crates" / "steelix-runtime"
PYTHON = (
    ROOT / ".venv" / "Scripts" / "python.exe"
    if sys.platform == "win32"
    else ROOT / ".venv" / "bin" / "python"
)
E2E_DIR = ROOT / "target" / "e2e"

CASES = sorted(p.stem for p in CASES_DIR.glob("*.py"))

# The test suites of the projects under `tests/projects/` run against the
# transpiled programs (`steelix test`, `test_projects.py`), not as steelix's.
collect_ignore = ["projects/skime/tests"]


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--update-golden",
        action="store_true",
        default=False,
        help="rewrite tests/golden/*.rs from the current transpiler output",
    )


@pytest.fixture
def update_golden(request: pytest.FixtureRequest) -> bool:
    return bool(request.config.getoption("--update-golden"))


def extension_library(name: str) -> Path:
    """The cdylib cargo built for the extension module crate `name`
    (`cargo_env(extension=True)`), named as the platform names libraries."""
    if sys.platform == "win32":
        file = f"{name}.dll"
    elif sys.platform == "darwin":
        file = f"lib{name}.dylib"
    else:
        file = f"lib{name}.so"
    return E2E_DIR / "target-ext" / "debug" / file


# The file name suffix that installs an extension module: `fib.so`, or
# `fib.pyd` on Windows.
EXTENSION_SUFFIX = ".pyd" if sys.platform == "win32" else ".so"


def cargo_env(*, extension: bool) -> dict[str, str]:
    """Environment for building generated crates.

    `PYO3_PYTHON` must be explicit: `.cargo/config.toml` of this repository does
    not apply to crates generated elsewhere. Extension modules and executables
    use separate target directories because `PYO3_BUILD_EXTENSION_MODULE`
    changes how pyo3-ffi links, which would otherwise rebuild PyO3 each time.
    Incremental compilation and debug info are off to keep the target directory
    small: every generated crate is built once, so the incremental cache is never
    reused, and debug info roughly doubles the size of PyO3 and the runtime.
    Set `CARGO_PROFILE_DEV_DEBUG` to override when a Rust backtrace needs lines.
    """
    env = dict(os.environ)
    env["PYO3_PYTHON"] = str(PYTHON)
    env["CARGO_INCREMENTAL"] = "0"
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    env["CARGO_TARGET_DIR"] = str(E2E_DIR / ("target-ext" if extension else "target"))
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    if extension:
        env["PYO3_BUILD_EXTENSION_MODULE"] = "1"
    return env


@dataclass
class Workspace:
    root: Path
    members: list[str]

    def cargo(self, *args: str, extension: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["cargo", *args],
            cwd=self.root,
            env=cargo_env(extension=extension),
            capture_output=True,
            text=True,
            check=False,
        )


def make_workspace(
    name: str,
    cases: list[str],
    mode: Mode,
    *,
    from_golden: bool,
    source_dir: Path = CASES_DIR,
    traceback_lines: bool = False,
) -> Workspace:
    """Generate one crate per case under a shared cargo workspace.

    With `from_golden` the generated module file is replaced by the golden
    snapshot, so that the snapshots themselves are what gets compiled.
    `traceback_lines` emits the `frame.line(n)` markers, for tests that
    compare tracebacks with CPython's line for line (design §9).
    """
    root = E2E_DIR / name
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    options = CrateOptions(
        mode=mode, runtime_path=RUNTIME, standalone=False, traceback_lines=traceback_lines
    )
    for case in cases:
        parsed = parse_file(source_dir / f"{case}.py")
        crate = root / case
        emit_crate(parsed, crate, options)
        if from_golden:
            module_file = rust_module_name(case).removeprefix("r#") + ".rs"
            shutil.copyfile(GOLDEN_DIR / f"{case}.rs", crate / "src" / module_file)
    members = ", ".join(f'"{case}"' for case in cases)
    manifest = f'[workspace]\nresolver = "3"\nmembers = [{members}]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest, encoding="utf-8")
    # Reuse the repository lock file so PyO3 resolves to the same version offline.
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    return Workspace(root, cases)


def compiler_warnings(
    cargo_json_output: str, workspace: Workspace, levels: tuple[str, ...] = ("warning",)
) -> list[str]:
    """Rendered diagnostics (warnings, or the given `levels`) that rustc or
    clippy reported for the workspace's own crates."""
    warnings = []
    for line in cargo_json_output.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-message":
            continue
        manifest = message.get("manifest_path", "")
        if not manifest.startswith(str(workspace.root)):
            continue
        diagnostic = message["message"]
        if diagnostic["level"] in levels and diagnostic["spans"]:
            warnings.append(diagnostic["rendered"])
    return warnings
