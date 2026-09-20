"""Restricted host CLI for the Vita USB serial debug transport."""

from __future__ import annotations

import argparse
import struct
import sys
from dataclasses import dataclass

MAGIC = b"VAD1"
VERSION = 1
MAX_PAYLOAD = 1024
OP_PING = 0x01
OP_INFO = 0x02


@dataclass(frozen=True)
class Frame:
    operation: int
    payload: bytes = b""


def encode(frame: Frame) -> bytes:
    if not 0 <= frame.operation <= 0xFF:
        raise ValueError("invalid operation")
    if len(frame.payload) > MAX_PAYLOAD:
        raise ValueError("payload exceeds protocol limit")
    return MAGIC + bytes((VERSION, frame.operation)) + struct.pack("<H", len(frame.payload)) + frame.payload


def decode(raw: bytes) -> Frame:
    if len(raw) < 8 or raw[:4] != MAGIC or raw[4] != VERSION:
        raise ValueError("invalid vita-adb frame")
    size = struct.unpack("<H", raw[6:8])[0]
    if size > MAX_PAYLOAD or len(raw) != 8 + size:
        raise ValueError("invalid vita-adb frame size")
    return Frame(raw[5], raw[8:])


def serial_module():
    try:
        import serial  # type: ignore
        from serial.tools import list_ports  # type: ignore
    except ImportError as error:
        raise SystemExit("Instala pyserial: py -m pip install -r vita-adb/host/requirements.txt") from error
    return serial, list_ports


def read_exact(port, amount: int) -> bytes:
    data = port.read(amount)
    if len(data) != amount:
        raise TimeoutError("la Vita no respondió dentro del tiempo permitido")
    return data


def request(port_name: str, operation: int) -> Frame:
    serial, _ = serial_module()
    with serial.Serial(port_name, baudrate=115200, timeout=3, write_timeout=3) as port:
        port.write(encode(Frame(operation)))
        header = read_exact(port, 8)
        length = struct.unpack("<H", header[6:8])[0]
        return decode(header + read_exact(port, length))


def main() -> int:
    parser = argparse.ArgumentParser(description="Vita USB diagnostic transport")
    parser.add_argument("--port", help="Puerto COM asignado a PS Vita Type D")
    parser.add_argument("command", choices=("devices", "ping", "info"))
    args = parser.parse_args()

    if args.command == "devices":
        _, list_ports = serial_module()
        for port in list_ports.comports():
            print(f"{port.device}\t{port.description}\t{port.hwid}")
        return 0
    if not args.port:
        parser.error("--port es obligatorio para este comando")
    opcode = OP_PING if args.command == "ping" else OP_INFO
    response = request(args.port, opcode)
    expected = opcode | 0x80
    if response.operation != expected:
        raise SystemExit(f"respuesta inesperada: 0x{response.operation:02x}")
    print(response.payload.decode("utf-8", errors="replace"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
