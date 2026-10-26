#!/usr/bin/env python3
"""Kahden erillisen demo-GUI:n LAN- ja palautumiskoe, ilman oikeita kilpailutietoja."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import time


def run(app, output):
    output.mkdir(parents=True, exist_ok=True)
    folder = Path(tempfile.mkdtemp(prefix="lan-", dir=output)).resolve()
    processes, logs = [], {}
    environment = dict(
        os.environ,
        SLINT_BACKEND="winit-software",
        SLINT_SCALE_FACTOR="1",
        AS2026_LAN_GUI_DIR=str(folder),
        AS2026_LAN_GUI_RECOVERY="1",
        AS2026_PRINT_PREVIEW_DIR=str(folder / "tulostusesikatselu"),
    )
    try:
        for role in ("isanta", "asiakas"):
            logs[role] = open(folder / (role + ".log"), "wb")
            processes.append(subprocess.Popen(
                [str(app), "--demo"],
                env=dict(environment, AS2026_LAN_GUI_ROLE=role),
                stdout=logs[role], stderr=subprocess.STDOUT,
            ))
        deadline = time.monotonic() + 70
        while time.monotonic() < deadline:
            failures = list(folder.glob("*-virhe.txt"))
            if failures:
                raise RuntimeError("; ".join(p.read_text(encoding="utf-8") for p in failures))
            if any(p.poll() is not None for p in processes):
                raise RuntimeError("GUI päättyi ennen LAN-kokeen valmistumista")
            if all("automaattinen palautuminen läpäisivät" in
                   (folder / (role + ".log")).read_text(encoding="utf-8", errors="replace")
                   for role in ("isanta", "asiakas")):
                break
            time.sleep(0.25)
        else:
            raise RuntimeError("Kahden GUI:n LAN-koe aikakatkaistiin")
        # Molemmat käyttävät samaa Project::to_toml-tallennusmuotoa.
        if (folder / "isanta-palautunut.toml").read_bytes() != (
                folder / "asiakas-palautunut.toml").read_bytes():
            raise RuntimeError("Isännän ja asiakkaan palautuneet kilpailutiedot eroavat")
        print("Kahden GUI:n LAN-koe läpäisty: Emit, käsikellot, pisteet, hyväksyntä, "
              "tulostusesikatselu, yhteyskatko, pysyvä jono ja palautuminen.")
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
        for process in processes:
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        for log in logs.values():
            log.close()
        print("LAN-kokeen lokit ja kuvat:", folder)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run(args.app.resolve(strict=True), args.output.resolve())
