import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from vita_adb import Frame, OP_INFO, decode, decode_tcp, encode, encode_tcp


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

    def test_authenticated_round_trip(self):
        key = bytes.fromhex("00" * 32)
        raw = encode_tcp(Frame(OP_INFO, b"status"), 42, key)
        self.assertEqual(decode_tcp(raw, key), (Frame(OP_INFO, b"status"), 42))

    def test_authenticated_rejects_tampering(self):
        key = bytes.fromhex("11" * 32)
        raw = bytearray(encode_tcp(Frame(OP_INFO), 7, key))
        raw[-1] ^= 1
        with self.assertRaises(ValueError):
            decode_tcp(bytes(raw), key)


if __name__ == "__main__":
    unittest.main()
