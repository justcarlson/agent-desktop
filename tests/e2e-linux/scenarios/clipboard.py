from __future__ import annotations

import os
import signal

from oracle import clipboard as oracle_clipboard
from oracle import pixels
from runner.legs import implementation
from runner.verdict import LegContext

TEXT_FORMAT = "text/plain"


def text_of(data: dict) -> str | None:
    if data.get("type") == "text":
        return data.get("text") or ""
    if data.get("found") is False:
        return None
    return None


def stop_owner(pid: int) -> None:
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        return


def _set_text(ctx: LegContext, text: str) -> None:
    envelope = ctx.product(["clipboard-set", text])
    if not envelope.ok:
        raise AssertionError(f"clipboard-set failed: {envelope.error}")


def _get(ctx: LegContext, fmt: str = "text") -> dict:
    envelope = ctx.product(["clipboard-get", "--format", fmt])
    if not envelope.ok:
        raise AssertionError(f"clipboard-get failed: {envelope.error}")
    return envelope.data or {}


def _small_image() -> pixels.Image:
    return pixels.Image(
        width=2,
        height=2,
        rgb=bytes((255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255)),
    )


@implementation("clipboard.text_roundtrip")
def text_roundtrip(ctx: LegContext) -> None:
    ctx.before("clipboard.owner", oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT))
    _set_text(ctx, "product text")
    ctx.after("clipboard.product_set", _get(ctx).get("text"))
    if _get(ctx).get("text") != "product text":
        raise AssertionError("the product did not read back the text it set")
    owner = oracle_clipboard.foreign_set(ctx.tier, TEXT_FORMAT, b"foreign text")
    try:
        ctx.before("clipboard.foreign_owner_pid", owner)
        read = oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT)
        ctx.after("clipboard.foreign_read", read)
        if read != b"foreign text":
            raise AssertionError("a foreign producer's text did not round trip")
        if text_of(_get(ctx)) != "foreign text":
            raise AssertionError("the product did not read the foreign text")
    finally:
        stop_owner(owner)


@implementation("clipboard.html_roundtrip")
def html_roundtrip(ctx: LegContext) -> None:
    payload = b"<b>foreign</b>"
    owner = oracle_clipboard.foreign_set(ctx.tier, "text/html", payload)
    try:
        ctx.before("clipboard.html_owner_pid", owner)
        read = oracle_clipboard.foreign_get(ctx.tier, "text/html")
        ctx.after("clipboard.html_read", read)
        if read != payload:
            raise AssertionError("the html selection did not round trip")
        auto = _get(ctx, "auto")
        ctx.after("clipboard.product_auto", auto.get("text"))
        if auto.get("text") is not None and "foreign" not in (auto.get("text") or ""):
            raise AssertionError("the product's auto read missed the html text")
    finally:
        stop_owner(owner)


@implementation("clipboard.image_roundtrip")
def image_roundtrip(ctx: LegContext) -> None:
    image = _small_image()
    payload = pixels.encode_png(image)
    owner = oracle_clipboard.foreign_set(ctx.tier, "image/png", payload)
    try:
        ctx.before("clipboard.image_owner_pid", owner)
        data = _get(ctx, "image")
        read_path = data.get("path")
        if not read_path:
            raise AssertionError("the product did not read the foreign image")
        read_image = pixels.decode_png(open(read_path, "rb").read())
        ctx.after("clipboard.image_bytes", (read_image.width, read_image.height))
        if not pixels.similar(read_image, image, tolerance=0.0):
            raise AssertionError("the image read back differs from the foreign image")
        set_envelope = ctx.product(["clipboard-set", "--image", str(_write_png(ctx, payload))])
        if not set_envelope.ok:
            raise AssertionError(f"clipboard-set --image failed: {set_envelope.error}")
        foreign = oracle_clipboard.foreign_get(ctx.tier, "image/png")
        ctx.after("clipboard.foreign_image", len(foreign or b""))
        if not foreign:
            raise AssertionError("a foreign reader saw no image after the product set one")
        if not pixels.similar(pixels.decode_png(foreign), image, tolerance=0.0):
            raise AssertionError("the foreign image reader decoded different pixels")
    finally:
        stop_owner(owner)


def _write_png(ctx: LegContext, payload: bytes):
    path = ctx.tier.runtime_dir / "clipboard-product.png"
    path.write_bytes(payload)
    return path


@implementation("clipboard.owner_outlives_setter")
def owner_outlives_setter(ctx: LegContext) -> None:
    ctx.before("clipboard.before_set", oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT))
    _set_text(ctx, "survives the setter")
    read = oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT)
    ctx.after("clipboard.after_setter_exit", read)
    if read != b"survives the setter":
        raise AssertionError("the selection owner did not outlive the setter")


@implementation("clipboard.clear_leaves_empty")
def clear_leaves_empty(ctx: LegContext) -> None:
    _set_text(ctx, "to be cleared")
    ctx.before("clipboard.before_clear", text_of(_get(ctx)))
    cleared = ctx.product(["clipboard-clear"])
    if not cleared.ok:
        raise AssertionError(f"clipboard-clear failed: {cleared.error}")
    foreign = oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT)
    ctx.after("clipboard.foreign_after_clear", foreign)
    if foreign:
        raise AssertionError("a foreign reader still saw text after clipboard-clear")


@implementation("clipboard.foreign_read_after_cli_exit")
def foreign_read_after_cli_exit(ctx: LegContext) -> None:
    ctx.before("clipboard.before_set", oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT))
    _set_text(ctx, "after exit")
    read = oracle_clipboard.foreign_get(ctx.tier, TEXT_FORMAT)
    ctx.after("clipboard.after_exit", read)
    if read != b"after exit":
        raise AssertionError("a foreign read after the CLI exited did not see the selection")
