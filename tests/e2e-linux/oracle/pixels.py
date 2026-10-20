from __future__ import annotations

from dataclasses import dataclass


@dataclass
class Image:
    width: int
    height: int
    rgb: bytes


def decode_xwd(data: bytes) -> Image:
    raise NotImplementedError("decode_xwd")


def decode_png(data: bytes) -> Image:
    raise NotImplementedError("decode_png")


def similar(
    a: Image,
    b: Image,
    region: tuple[int, int, int, int] | None = None,
    *,
    tolerance: float = 0.02,
) -> bool:
    raise NotImplementedError("similar")
