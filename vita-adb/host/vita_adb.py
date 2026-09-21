"""Restricted host CLI for Vita USB serial and authenticated Wi-Fi diagnostics."""

from __future__ import annotations

import argparse
import hashlib
import hmac
import os
import socket
import struct
import sys
from dataclasses import dataclass

MAGIC = b"VAD1"
TCP_MAGIC = b"VAD2"
VERSION = 1
MAX_PAYLOAD = 1024
TCP_MAX_PAYLOAD = 512
TCP_HEADER = struct.Struct("<4sBBIH")
TCP_TAG_SIZE = 32
TCP_PORT = 39999
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


def encode_tcp(frame: Frame, sequence: int, key: bytes) -> bytes:
    if not 0 <= sequence <= 0xFFFFFFFF:
        raise ValueError("invalid sequence")
    if not 0 <= frame.operation <= 0xFF or len(frame.payload) > TCP_MAX_PAYLOAD:
        raise ValueError("invalid authenticated frame")
    header = TCP_HEADER.pack(TCP_MAGIC, VERSION, frame.operation, sequence, len(frame.payload))
    raw = header + frame.payload
    return raw + hmac.new(key, raw, hashlib.sha256).digest()


def decode_tcp(raw: bytes, key: bytes) -> tuple[Frame, int]:
    if len(raw) < TCP_HEADER.size + TCP_TAG_SIZE:
        raise ValueError("authenticated frame too short")
    magic, version, operation, sequence, size = TCP_HEADER.unpack(raw[:TCP_HEADER.size])
    if magic != TCP_MAGIC or version != VERSION or size > TCP_MAX_PAYLOAD:
        raise ValueError("invalid authenticated frame")
    expected_length = TCP_HEADER.size + size + TCP_TAG_SIZE
    if len(raw) != expected_length:
        raise ValueError("invalid authenticated frame size")
    signed = raw[:-TCP_TAG_SIZE]
    if not hmac.compare_digest(hmac.new(key, signed, hashlib.sha256).digest(), raw[-TCP_TAG_SIZE:]):
        raise ValueError("invalid authenticated frame signature")
    return Frame(operation, raw[TCP_HEADER.size:-TCP_TAG_SIZE]), sequence


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


def read_socket_exact(connection: socket.socket, amount: int) -> bytes:
    data = bytearray()
    while len(data) < amount:
        piece = connection.recv(amount - len(data))
        if not piece:
            raise ConnectionError("la Vita cerró la conexión antes de responder")
        data.extend(piece)
    return bytes(data)


def pairing_key(value: str) -> bytes:
    if len(value) != 64:
        raise ValueError("la clave de emparejamiento debe tener 64 caracteres hexadecimales")
    try:
        return bytes.fromhex(value)
    except ValueError as error:
        raise ValueError("la clave de emparejamiento no es hexadecimal") from error


def request_tcp(host: str, operation: int, key: bytes, port: int = TCP_PORT) -> Frame:
    sequence = int.from_bytes(os.urandom(4), "little")
    request_frame = encode_tcp(Frame(operation), sequence, key)
    with socket.create_connection((host, port), timeout=5) as connection:
        connection.settimeout(5)
        connection.sendall(request_frame)
        header = read_socket_exact(connection, TCP_HEADER.size)
        _, _, _, _, size = TCP_HEADER.unpack(header)
        if size > TCP_MAX_PAYLOAD:
            raise ValueError("respuesta TCP demasiado grande")
        raw = header + read_socket_exact(connection, size + TCP_TAG_SIZE)
    response, response_sequence = decode_tcp(raw, key)
    if response_sequence != sequence:
        raise ValueError("la respuesta no corresponde a esta solicitud")
    return response


def main() -> int:
    parser = argparse.ArgumentParser(description="Vita diagnostic transport")
    parser.add_argument("--port", help="Puerto COM asignado a PS Vita Type D")
    parser.add_argument("--host", help="IP de la Vita en la red local (transporte autenticado)")
    parser.add_argument("--tcp-port", type=int, default=TCP_PORT, help=f"Puerto TCP (predeterminado: {TCP_PORT})")
    parser.add_argument("--key", default=os.environ.get("VITA_ADBD_KEY"), help="Clave hexadecimal o variable VITA_ADBD_KEY")
    parser.add_argument("command", choices=("devices", "ping", "info"))
    args = parser.parse_args()

    if args.command == "devices":
        _, list_ports = serial_module()
        for port in list_ports.comports():
            print(f"{port.device}\t{port.description}\t{port.hwid}")
        return 0
    opcode = OP_PING if args.command == "ping" else OP_INFO
    if args.host:
        if not args.key:
            parser.error("--key o la variable VITA_ADBD_KEY es obligatoria con --host")
        response = request_tcp(args.host, opcode, pairing_key(args.key), args.tcp_port)
    elif args.port:
        response = request(args.port, opcode)
    else:
        parser.error("--host o --port es obligatorio para este comando")
    expected = opcode | 0x80
    if response.operation != expected:
        raise SystemExit(f"respuesta inesperada: 0x{response.operation:02x}")
    print(response.payload.decode("utf-8", errors="replace"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
