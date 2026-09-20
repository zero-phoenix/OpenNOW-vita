import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from vita_adb import Frame, OP_INFO, decode, encode


class ProtocolTests(unittest.TestCase):
    def test_round_trip(self):
        frame = Frame(OP_INFO, b"status")
        self.assertEqual(decode(encode(frame)), frame)

    def test_rejects_wrong_magic(self):
        with self.assertRaises(ValueError):
            decode(b"BAD!\x01\x01\x00\x00")

    def test_rejects_size_mismatch(self):
        with self.assertRaises(ValueError):
            decode(b"VAD1\x01\x01\x02\x00x")


if __name__ == "__main__":
    unittest.main()
