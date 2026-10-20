from __future__ import annotations

import base64
import subprocess

from oracle import atspi_raw, pixels, x11
from runner import apps
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, longest_text, live_root, ref_for, window_of

REFUSAL_CODES = {"ACTION_FAILED", "ACTION_NOT_SUPPORTED", "TIMEOUT", "WINDOW_NOT_FOUND", "PLATFORM_NOT_SUPPORTED"}


def match_window_line(lines: list[str], title: str) -> str | None:
    for line in lines:
        parts = line.split(None, 4)
        if len(parts) < 4 or not parts[0].startswith("0x"):
            continue
        if len(parts) == 5:
            found = parts[4]
        else:
            host, _, found = parts[3].partition(" ")
        if found.strip() == title:
            return parts[0]
    return None


def non_blank(image: pixels.Image) -> bool:
    first = image.rgb[:3]
    return any(image.rgb[offset : offset + 3] != first for offset in range(3, len(image.rgb), 3))


def capture_bytes(envelope) -> tuple[bytes, str]:
    data = assert_ok(envelope)
    payload = data.get("data")
    if not payload:
        raise AssertionError(f"{envelope.argv} returned no image data")
    return base64.b64decode(payload), data.get("format") or "png"


def decode(data: bytes, image_format: str) -> pixels.Image:
    if image_format.lower() in ("xwd", "x-window-dump"):
        return pixels.decode_xwd(data)
    return pixels.decode_png(data)


def _window_title(ctx: LegContext, window: str) -> str:
    data = assert_ok(ctx.product(["list-windows"]))
    stack = [data]
    while stack:
        item = stack.pop()
        if isinstance(item, dict):
            if item.get("id") == window and item.get("title"):
                return item["title"]
            stack.extend(item.values())
        elif isinstance(item, list):
            stack.extend(item)
    raise AssertionError(f"no title for window {window}")


def _x11_id(ctx: LegContext, window: str) -> str:
    title = _window_title(ctx, window)
    process = subprocess.run(
        ["wmctrl", "-l"], capture_output=True, text=True, env=ctx.tier.env, timeout=5, check=False
    )
    found = match_window_line(process.stdout.splitlines(), title)
    if not found:
        active = x11.active_window(ctx.tier)
        if not active:
            raise AssertionError(f"no X11 window matches {title!r}")
        return active
    return found


def _xwd(ctx: LegContext, x11_window: str) -> bytes:
    process = subprocess.run(
        ["xwd", "-id", x11_window, "-silent"],
        capture_output=True,
        env=ctx.tier.env,
        timeout=10,
        check=False,
    )
    if process.returncode != 0 or not process.stdout:
        raise AssertionError("xwd returned no bytes")
    return process.stdout


def _editor(ctx: LegContext):
    app = apps.launch(ctx.tier, "Text Editor")
    window = window_of(app)
    assert_ok(ctx.product(["type", ref_for(ctx, window, role="text"), "capture baseline"]))
    atspi_raw.poll(lambda: "capture baseline" in longest_text(live_root(ctx.tier, app)), 6.0)
    return app, window


@implementation("capture.window_pixels_match_xwd")
def window_pixels_match_xwd(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        product = decode(payload, image_format)
        oracle_image = pixels.decode_xwd(_xwd(ctx, _x11_id(ctx, window)))
        ctx.before("pixels.product", (product.width, product.height))
        ctx.after("pixels.xwd", (oracle_image.width, oracle_image.height))
        if not pixels.similar(product, oracle_image, tolerance=0.05):
            raise AssertionError("the product capture does not match the xwd pixels")
    finally:
        apps.stop(app)


@implementation("capture.stable_at_idle")
def stable_at_idle(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        first, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        first_image = decode(first, image_format)
        ctx.before("pixels.first", (first_image.width, first_image.height))
        second, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        second_image = decode(second, image_format)
        ctx.after("pixels.second", (second_image.width, second_image.height))
        if not pixels.similar(first_image, second_image, tolerance=0.0):
            raise AssertionError("two idle captures of the same window differ")
    finally:
        apps.stop(app)


@implementation("capture.content_change_moves_region")
def content_change_moves_region(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        before_payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        before = decode(before_payload, image_format)
        ctx.before("pixels.content", (before.width, before.height))
        assert_ok(ctx.product(["type", ref_for(ctx, window, role="text"), "changed content that moves pixels"]))
        after_payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        after = decode(after_payload, image_format)
        ctx.after("pixels.content", (after.width, after.height))
        if pixels.similar(before, after, tolerance=0.01):
            raise AssertionError("a content change did not move the compared region")
    finally:
        apps.stop(app)


@implementation("capture.covered_window_capture")
def covered_window_capture(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    cover = apps.launch(ctx.tier, "Calculator")
    try:
        x, y, _, _ = x11.geometry(ctx.tier, window)
        assert_ok(ctx.product(["move-window", "--window-id", window_of(cover), "--x", str(x), "--y", str(y)]))
        atspi_raw.poll(lambda: x11.geometry(ctx.tier, window_of(cover))[0] == x, 6.0)
        payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        product = decode(payload, image_format)
        oracle_image = pixels.decode_xwd(_xwd(ctx, _x11_id(ctx, window)))
        ctx.before("pixels.covered", (product.width, product.height))
        ctx.after("pixels.covered_oracle", (oracle_image.width, oracle_image.height))
        if not pixels.similar(product, oracle_image, tolerance=0.05):
            raise AssertionError("the covered window did not capture its own content")
    finally:
        apps.stop(cover)
        apps.stop(app)


@implementation("capture.minimised_window_capture")
def minimised_window_capture(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
        before = decode(payload, image_format)
        ctx.before("pixels.minimised", (before.width, before.height))
        assert_ok(ctx.product(["minimize", "--window-id", window]))
        envelope = ctx.product(["screenshot", "--window-id", window])
        if not envelope.ok:
            code = (envelope.error or {}).get("code")
            if code not in REFUSAL_CODES:
                raise AssertionError(f"a minimised capture refused with {code!r}")
            ctx.after("pixels.minimised", code)
            return
        after = decode(capture_bytes(envelope)[0], (envelope.data or {}).get("format") or image_format)
        ctx.after("pixels.minimised", (after.width, after.height))
        if ctx.tier.name == "x11-mutter":
            ctx.finding("the minimised window was still capturable under mutter")
        else:
            raise AssertionError("a minimised window captured content")
    finally:
        apps.stop(app)


@implementation("capture.display_capture")
def display_capture(ctx: LegContext) -> None:
    envelope = ctx.product(["screenshot", "--screen", "0"])
    payload, image_format = capture_bytes(envelope)
    image = decode(payload, image_format)
    ctx.before("pixels.display", (image.width, image.height))
    ctx.after("pixels.display_content", non_blank(image))
    if not non_blank(image):
        raise AssertionError("the display capture decoded to a blank image")


@implementation("capture.wayland_route_gated")
def wayland_route_gated(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Text Editor")
    try:
        envelope = ctx.product(["screenshot", "--window-id", window_of(app)])
        if envelope.ok:
            raise AssertionError("the gated Wayland capture route answered")
        code = (envelope.error or {}).get("code")
        disposition = (envelope.error or {}).get("disposition") or {}
        ctx.before("capture.refusal", code)
        ctx.after("capture.disposition", disposition.get("delivery"))
        if code not in REFUSAL_CODES:
            raise AssertionError(f"the gated route answered {code!r}")
        if disposition.get("delivery") != "not_delivered":
            raise AssertionError("the gated route did not report not delivered")
    finally:
        apps.stop(app)
