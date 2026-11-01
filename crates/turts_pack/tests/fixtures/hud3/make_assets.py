#!/usr/bin/env python3
"""Generate small header-valid test assets using only Python's standard library."""
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).parent


def png(width: int, height: int) -> bytes:
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    chunk = struct.pack(">I", len(ihdr)) + b"IHDR" + ihdr
    chunk += struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk


def ogg_vorbis(seconds: int = 1) -> bytes:
    identification = b"\x01vorbis" + struct.pack("<I", 0) + bytes([2])
    identification += struct.pack("<I", 48_000) + struct.pack("<iii", 0, 0, 0)
    identification += bytes([0xB8, 1])  # block sizes and framing bit

    def page(flags: int, granule: int, sequence: int, packet: bytes = b"") -> bytes:
        lacing = bytes([len(packet)]) if packet else b""
        header = (b"OggS\0" + bytes([flags]) + struct.pack("<QII", granule, 1, sequence)
                  + bytes(4) + bytes([len(lacing)]) + lacing)
        body = header + packet
        crc = 0
        for byte in body:
            crc ^= byte << 24
            for _ in range(8):
                crc = ((crc << 1) ^ 0x04C11DB7) & 0xFFFFFFFF if crc & 0x80000000 else (crc << 1) & 0xFFFFFFFF
        return body[:22] + struct.pack("<I", crc) + body[26:]

    return page(0x02, 0, 0, identification) + page(0x04, 48_000 * seconds, 1)


(ROOT / "assets").mkdir(exist_ok=True)
(ROOT / "assets/icon.png").write_bytes(png(32, 32))
(ROOT / "assets/cursor.png").write_bytes(png(32, 32))
(ROOT / "assets/font.ttf").write_bytes(b"\x00\x01\x00\x00" + bytes(24))
(ROOT / "assets/font.otf").write_bytes(b"OTTO" + bytes(24))
(ROOT / "assets/alert.ogg").write_bytes(ogg_vorbis())
