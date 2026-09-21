import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from vita_monitor import report_names


class MonitorTests(unittest.TestCase):
    def test_keeps_only_completed_report_directories(self):
        lines = [
            "drwxr-xr-x 1 vita vita 0 Sep 20 12:00 report-1-a",
            "drwxr-xr-x 1 vita vita 0 Sep 20 12:00 report-2-b.partial",
            "-rw-r--r-- 1 vita vita 1 Sep 20 12:00 frame_stats.log",
        ]
        self.assertEqual(report_names(lines), ["report-1-a"])


if __name__ == "__main__":
    unittest.main()
