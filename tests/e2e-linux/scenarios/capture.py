from __future__ import annotations

import base64
import hashlib
import subprocess

from oracle import atspi_raw, pixels, x11
from runner import apps
from runner.helper import changed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.headed_actions import assert_ok, longest_text, live_root, ref_for, window_of
from scenarios.pointer import frame_extents

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
    return x11.xid(ctx.tier, window)


def _xwd(ctx: LegContext, x11_window: str | None) -> bytes:
    process = subprocess.run(
        ["xwd", *(["-id", x11_window] if x11_window else ["-root"]), "-silent"],
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
    assert_ok(ctx.product(["type", ref_for(ctx, window, role="textfield"), "capture baseline"]))
    atspi_raw.poll(lambda: "capture baseline" in longest_text(live_root(ctx.tier, app)), 6.0)
    return app, window


def _oracle_image(ctx, window=None):
    image = pixels.decode_xwd(_xwd(ctx, _x11_id(ctx, window) if window else None))
    if window:
        left, right, top, bottom = frame_extents(ctx.tier, window)
        width, height = image.width - left - right, image.height - top - bottom
        require(min(left, right, top, bottom) >= 0 and min(width, height) > 0,
                "the independently reported shadow does not fit the XWD image")
        return image.crop((left, top, width, height))
    return image


def _product_image(ctx, window):
    payload, image_format = capture_bytes(ctx.product(["screenshot", "--window-id", window]))
    return decode(payload, image_format)


def _signature(image):
    return image.width, image.height, hashlib.sha256(image.rgb).hexdigest()


def _matches(product, oracle):
    require(pixels.similar(product, oracle, tolerance=0.05),
            "the product capture does not match independent xwd pixels")


@implementation("capture.window_pixels_match_xwd")
def window_pixels_match_xwd(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        before = _oracle_image(ctx, window)
        ctx.before("pixels.window_dimensions", (before.width, before.height))
        product = _product_image(ctx, window)
        after = _oracle_image(ctx, window)
        ctx.after("pixels.window_dimensions", (after.width, after.height))
        _matches(product, after)
        final = _oracle_image(ctx, window)
        ctx.restored("pixels.window_dimensions", (final.width, final.height))
    finally:
        apps.stop(app)


def stable_at_idle(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        before = _oracle_image(ctx, window)
        ctx.before("pixels.idle", _signature(before))
        first, second = _product_image(ctx, window), _product_image(ctx, window)
        after = _oracle_image(ctx, window)
        ctx.after("pixels.idle", _signature(after))
        _matches(first, before)
        _matches(second, after)
        require(pixels.similar(first, second, tolerance=0.0), "two idle captures differ")
        ctx.restored("pixels.idle", _signature(_oracle_image(ctx, window)))
    finally:
        apps.stop(app)


def content_change_moves_region(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        text = longest_text(live_root(ctx.tier, app))
        before = _oracle_image(ctx, window)
        ctx.before("pixels.content", _signature(before))
        _matches(_product_image(ctx, window), before)
        try:
            assert_ok(ctx.product(["type", ref_for(ctx, window, role="textfield"),
                                   "changed content that moves pixels"]))
            product = _product_image(ctx, window)
            after = _oracle_image(ctx, window)
            ctx.after("pixels.content", _signature(after))
            _matches(product, after)
            require(not pixels.similar(before, after, tolerance=0.01),
                    "content change did not move independent xwd pixels")
        finally:
            target = ref_for(ctx, window, role="textfield")
            assert_ok(ctx.product(["clear", target]))
            if text:
                assert_ok(ctx.product(["type", target, text]))
            changed(lambda: longest_text(live_root(ctx.tier, app)), text)
            restored = changed(lambda: _signature(_oracle_image(ctx, window)),
                               _signature(before), seconds=3)
            ctx.restored("pixels.content", restored)
    finally:
        apps.stop(app)


def covered_window_capture(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    cover = None
    try:
        before = _oracle_image(ctx, window)
        x, y, _, _ = x11.geometry(ctx.tier, window)
        region = (x, y, before.width, before.height)
        root_before = _oracle_image(ctx).crop(region)
        stack_before = x11.stacking_xids(ctx.tier)
        ctx.before("pixels.covered_dimensions", (before.width, before.height))
        ctx.before("x11.covered_stack", stack_before)
        if ctx.tier.name == "x11-openbox":
            ctx.before("pixels.covered_root", _signature(root_before))
        cover = apps.launch(ctx.tier, "Calculator")
        assert_ok(ctx.product(["move-window", "--window-id", window_of(cover),
                               "--x", str(x), "--y", str(y)]))
        changed(lambda: x11.geometry(ctx.tier, window_of(cover))[:2], (x, y), seconds=6)
        target_xid, cover_xid = _x11_id(ctx, window), _x11_id(ctx, window_of(cover))
        stack_after = x11.stacking_xids(ctx.tier)
        ctx.after("x11.covered_stack", stack_after)
        require(target_xid in stack_after and cover_xid in stack_after
                and stack_after.index(cover_xid) > stack_after.index(target_xid),
                "the covering window is not above the target")
        cx, cy, cw, ch = x11.geometry(ctx.tier, window_of(cover))
        require(min(x + before.width, cx + cw) > max(x, cx)
                and min(y + before.height, cy + ch) > max(y, cy),
                "the covering window does not overlap the target")
        product, after = _product_image(ctx, window), _oracle_image(ctx, window)
        root_after = _oracle_image(ctx).crop(region)
        ctx.after("pixels.covered_dimensions", (after.width, after.height))
        if ctx.tier.name == "x11-openbox":
            ctx.after("pixels.covered_root", _signature(root_after))
        _matches(product, after)
        require(pixels.similar(before, after, tolerance=0.01), "by-id capture changed under cover")
        mismatch = pixels.mismatch_fraction(root_before, root_after)
        if ctx.tier.name == "x11-openbox":
            require(mismatch > 0.05, "root pixels do not prove that the cover is present")
        ctx.finding(f"covered root mismatch fraction: {mismatch:.4f}")
    finally:
        if cover is not None:
            apps.stop(cover)
            ctx.restored("x11.covered_stack", changed(
                lambda: x11.stacking_xids(ctx.tier), stack_before))
            if ctx.tier.name == "x11-openbox":
                restored = changed(lambda: _signature(_oracle_image(ctx).crop(region)),
                                   _signature(root_before), seconds=3)
                ctx.restored("pixels.covered_root", restored)
        final = _oracle_image(ctx, window)
        ctx.restored("pixels.covered_dimensions", (final.width, final.height))
        apps.stop(app)


def _minimized(ctx, window):
    from scenarios.lifecycle import _wm_state, is_hidden
    return is_hidden(_wm_state(ctx.tier, window))


def minimised_window_capture(ctx: LegContext) -> None:
    app, window = _editor(ctx)
    try:
        before = _minimized(ctx, window)
        require(not before, "capture target starts minimized")
        ctx.before("x11.minimized", before)
        baseline = _product_image(ctx, window)
        try:
            assert_ok(ctx.product(["minimize", "--window-id", window]))
            ctx.after("x11.minimized", changed(lambda: _minimized(ctx, window), True, seconds=6))
            response = ctx.product(["screenshot", "--window-id", window])
            if not response.ok:
                code = (response.error or {}).get("code")
                require(code in REFUSAL_CODES, f"minimized capture refused with {code!r}")
                ctx.finding(f"minimized capture refusal: {code}")
            else:
                captured = decode(*capture_bytes(response))
                _matches(captured, baseline)
                require(ctx.tier.name == "x11-mutter", "a minimized openbox window captured content")
                ctx.finding("the minimized window remained capturable under mutter")
        finally:
            assert_ok(ctx.product(["restore", "--window-id", window]))
            ctx.restored("x11.minimized", changed(lambda: _minimized(ctx, window), before, seconds=6))
            _matches(_product_image(ctx, window), _oracle_image(ctx, window))
    finally:
        apps.stop(app)


def display_capture(ctx: LegContext) -> None:
    app, _ = _editor(ctx)
    try:
        before = _oracle_image(ctx)
        ctx.before("pixels.display_dimensions", (before.width, before.height))
        response = ctx.product(["screenshot", "--screen", "0"])
        after = _oracle_image(ctx)
        ctx.after("pixels.display_dimensions", (after.width, after.height))
        if ctx.tier.name == "x11-mutter":
            error = response.error or {}
            require(not response.ok and error.get("code") == "PLATFORM_NOT_SUPPORTED"
                    and (error.get("details") or {}).get("reason") == "compositing",
                    "composited display capture did not report its landed refusal")
        else:
            product = decode(*capture_bytes(response))
            _matches(product, after)
            require(non_blank(product), "display capture decoded to a blank image")
        final = _oracle_image(ctx)
        ctx.restored("pixels.display_dimensions", (final.width, final.height))
    finally:
        apps.stop(app)


def wayland_route_gated(ctx: LegContext) -> None:
    app = apps.launch(ctx.tier, "Text Editor")
    try:
        envelope = ctx.product(["screenshot", "--window-id", window_of(app)])
        if envelope.ok:
            raise AssertionError("the gated Wayland capture route answered")
        code = (envelope.error or {}).get("code")
        disposition = (envelope.error or {}).get("disposition") or {}
        ctx.before("pixels.window_bounds", live_root(ctx.tier, app).extents())
        ctx.after("pixels.window_bounds", live_root(ctx.tier, app).extents())
        if code not in REFUSAL_CODES:
            raise AssertionError(f"the gated route answered {code!r}")
        if disposition.get("delivery") != "not_delivered":
            raise AssertionError("the gated route did not report not delivered")
    finally:
        apps.stop(app)
