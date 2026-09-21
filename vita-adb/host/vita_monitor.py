"""Local-only collector for OpenNOW Vita diagnostic evidence over VitaShell FTP."""

from __future__ import annotations

import argparse
import ftplib
import time
from datetime import datetime, timezone
from pathlib import Path


REPORTS_DIR = "ux0:data/opennow/reports"
INTERVAL_SECONDS = 15


def report_names(lines: list[str]) -> list[str]:
    """Return completed report directory names from VitaShell's POSIX LIST output."""
    names: list[str] = []
    for line in lines:
        parts = line.split(maxsplit=8)
        if len(parts) != 9 or not parts[0].startswith("d"):
            continue
        name = parts[8]
        if name.startswith("report-") and not name.endswith(".partial"):
            names.append(name)
    return sorted(names)


class VitaMonitor:
    def __init__(self, host: str, output: Path) -> None:
        self.host = host
        self.output = output
        self.log_path = output / "monitor.log"

    def note(self, message: str) -> None:
        timestamp = datetime.now(timezone.utc).isoformat(timespec="seconds")
        line = f"{timestamp} {message}"
        self.output.mkdir(parents=True, exist_ok=True)
        with self.log_path.open("a", encoding="utf-8") as log:
            log.write(line + "\n")
        print(line)

    def sync_once(self) -> int:
        downloaded = 0
        with ftplib.FTP(self.host, timeout=10) as ftp:
            ftp.login("anonymous", "anonymous")
            try:
                ftp.cwd(REPORTS_DIR)
            except ftplib.error_perm:
                self.note("waiting: OpenNOW has not created ux0:data/opennow/reports yet")
                return 0
            listing: list[str] = []
            ftp.retrlines("LIST", listing.append)
            for report in report_names(listing):
                ftp.cwd(report)
                files: list[str] = []
                ftp.retrlines("LIST", files.append)
                destination = self.output / report
                destination.mkdir(parents=True, exist_ok=True)
                for line in files:
                    parts = line.split(maxsplit=8)
                    if len(parts) != 9 or parts[0].startswith("d"):
                        continue
                    name = parts[8]
                    if name not in {"manifest.json", "screen.png", "opennow.log", "frame_stats.log"}:
                        continue
                    local = destination / name
                    if local.exists() and local.stat().st_size == int(parts[4]):
                        continue
                    with local.open("wb") as target:
                        ftp.retrbinary(f"RETR {name}", target.write)
                    downloaded += 1
                    self.note(f"saved {report}/{name}")
                ftp.cwd("..")
        if downloaded == 0:
            self.note("checked: no new diagnostic evidence")
        return downloaded


def main() -> int:
    parser = argparse.ArgumentParser(description="Collect OpenNOW Vita evidence locally via VitaShell FTP")
    parser.add_argument("--host", required=True, help="IP of the PS Vita running VitaShell FTP")
    parser.add_argument("--output", type=Path, default=Path("vita-evidence"), help="local evidence directory")
    parser.add_argument("--watch", action="store_true", help="keep collecting every 15 seconds")
    args = parser.parse_args()
    monitor = VitaMonitor(args.host, args.output)
    while True:
        try:
            monitor.sync_once()
            if not args.watch:
                return 0
        except (OSError, EOFError, ftplib.Error) as error:
            monitor.note(f"FTP unavailable: {error}")
            if not args.watch:
                return 1
        try:
            time.sleep(INTERVAL_SECONDS)
        except KeyboardInterrupt:
            monitor.note("monitor stopped by user")
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
