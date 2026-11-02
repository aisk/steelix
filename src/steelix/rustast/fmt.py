"""Run `rustfmt` over generated source.

rustfmt is mandatory: golden snapshots compare formatted output, so the edition
and style edition are pinned by `rustfmt.toml` next to this module instead of
being picked up from whatever directory steelix runs in.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

CONFIG = Path(__file__).with_name("rustfmt.toml")


class RustfmtError(RuntimeError):
    pass


def rustfmt(source: str) -> str:
    exe = shutil.which("rustfmt")
    if exe is None:
        raise RustfmtError("rustfmt not found on PATH; it is required to emit Rust code")
    proc = subprocess.run(
        [exe, "--emit", "stdout", "--config-path", str(CONFIG)],
        input=source,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RustfmtError(f"rustfmt failed:\n{proc.stderr}\n--- source ---\n{source}")
    return proc.stdout


def config_text() -> str:
    return CONFIG.read_text()
