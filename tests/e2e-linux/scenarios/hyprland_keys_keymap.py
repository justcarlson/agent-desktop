"""A keymap with a layout variant and options kept through virtual keyboard typing on the attached `hyprland` tier.

The leg sets `kb_layout us`, `kb_variant dvorak` and `kb_options
ctrl:nocaps` on the session compositor, then types into the key log and
presses Tab in the zoo. The configured options and every keyboard's layout,
variant, options and keymap name must read back as they were set, the app
must see each letter on the key the Dvorak variant puts it on, Tab must move
focus, and the original keymap must come back.

fcitx5's input-method keyboard copies the compositor keymap only after it
handles a key, so while the variant is set it may still read the keymap it
had before. After the restore the leg types one key so it copies the
original keymap again, and only then reads the keyboards back strictly.
"""

from __future__ import annotations

import contextlib

from oracle import atspi_raw as raw
from oracle import hyprland
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_keys import _focused_key_log, _typed
from scenarios.hyprland_keys_common import keyboards, presses
from scenarios.hyprland_keys_controls import focused_zoo, tab_moves_focus
from scenarios.parallel_zoo import IME_KEYBOARDS

OPTIONS = ("kb_layout", "kb_variant", "kb_options")
VARIANT = {"kb_layout": "us", "kb_variant": "dvorak", "kb_options": "ctrl:nocaps"}
TYPED = "Hello, dvorak"
DVORAK_CODES = {"h": 44, "e": 40}
SETTLE_S = 3.0
SYNC_KEY = "x"


def readback_problems(expected: dict, found: dict) -> list[str]:
    return [f"{field} reads {found.get(field)!r}, not {expected.get(field)!r}"
            for field in sorted(set(expected) | set(found)) if found.get(field) != expected.get(field)]


def ime_settled(read: dict, lagging: dict) -> dict:
    """`read` with each input-method keyboard that still reads as it did in `lagging` shown as the
    first other keyboard reads, since the input method copies a new keymap only after its next key."""
    old = {item[0]: item for item in lagging.get("keyboards", [])}
    found = read.get("keyboards", [])
    others = [item for item in found if item[0] not in IME_KEYBOARDS]
    if not others:
        return read
    settled = [(item[0], *others[0][1:]) if item[0] in IME_KEYBOARDS and old.get(item[0]) == item else item
               for item in found]
    return {**read, "keyboards": settled}


class Unrecorded:
    """The leg's context with its oracle reads dropped, for a step the leg's channels already cover."""

    def __init__(self, ctx: LegContext):
        self._ctx = ctx

    def __getattr__(self, name: str):
        return getattr(self._ctx, name)

    def before(self, *_args) -> None:
        return None

    after = restored = before


def resync_input_method(ctx: LegContext) -> None:
    """Types one key into a fresh key log so the input-method keyboard copies the compositor keymap."""
    quiet = Unrecorded(ctx)
    with _focused_key_log(quiet, "keymap-sync") as (app, _log, ref):
        problems = _typed(quiet, app, ref, SYNC_KEY, SYNC_KEY)
    if problems:
        ctx.finding(f"the resync key after the restore: {'; '.join(problems)}")


def placement_problems(found: list[dict], codes: dict[str, int]) -> list[str]:
    """Each letter's presses arrive on the hardware code the layout variant gives it."""
    problems = []
    for letter, code in codes.items():
        seen = {item.get("code") for item in found
                if item.get("event") == "press" and str(item.get("keysym", "")).lower() == letter}
        if seen != {code}:
            problems.append(f"{letter} arrived on codes {sorted(seen)}, not [{code}]")
    return problems


def _option(tier, name: str) -> str | None:
    main = main_instance(tier)
    with contextlib.suppress(ValueError, RuntimeError, OSError):
        found = hyprland.query(shared(tier), main["signature"], f"getoption input:{name}", pid=main["pid"])
        if isinstance(found, dict) and isinstance(found.get("str"), str):
            return found["str"]
    return None


def keymap_read(tier) -> dict:
    """The configured keymap options and each keyboard's layout, variant, options and keymap name."""
    main = main_instance(tier)
    found = hyprland.query(shared(tier), main["signature"], "devices", pid=main["pid"]).get("keyboards", [])
    read: dict = {name: _option(tier, name) for name in OPTIONS}
    read["keyboards"] = sorted((item.get("name"), item.get("layout"), item.get("variant"), item.get("options"),
                                item.get("active_keymap")) for item in found)
    return read


def _set_options(tier, values: dict[str, str]) -> None:
    main = main_instance(tier)
    runtime, signature = shared(tier), main["signature"]
    if hyprland.uses_lua(runtime, signature):
        fields = ", ".join(f"{name} = {hyprland.lua_string(values[name])}" for name in OPTIONS)
        commands = [["eval", f"hl.config({{ input = {{ {fields} }} }})"]]
    else:
        commands = [["keyword", f"input:{name}", hyprland.lua_string(values[name])[1:-1]] for name in OPTIONS]
    for argv in commands:
        output = hyprland.control(runtime, signature, argv)
        require(output == "ok", f"the compositor refused {argv}: {output}")


def _carries(read: dict, values: dict[str, str]) -> bool:
    return any((layout, variant, options) == tuple(values[name] for name in OPTIONS)
               for _, layout, variant, options, _ in read["keyboards"])


@contextlib.contextmanager
def variant_keymap(tier, original: dict):
    """The variant keymap set on the compositor, and the original options set back on the way out."""
    values = {name: original[name] if original[name] is not None else "" for name in OPTIONS}
    _set_options(tier, VARIANT)
    try:
        require(raw.poll(lambda: _carries(keymap_read(tier), VARIANT), SETTLE_S),
                f"no keyboard took the variant keymap: {keymap_read(tier)}")
        yield keymap_read(tier)
    finally:
        _set_options(tier, values)


@implementation("input.hyprland_keymap_variant_options_preserved")
def keymap_variant_options_preserved(ctx: LegContext) -> None:
    tier = ctx.tier
    original, before = keymap_read(tier), keyboards(tier)
    require(None not in (original[name] for name in OPTIONS), f"the keymap options cannot be read: {original}")
    ctx.before("hyprland.keymap", original)
    problems: list[str] = []
    with variant_keymap(tier, original) as during:
        problems += readback_problems({**during, **VARIANT}, during)
        with _focused_key_log(ctx, "keymap") as (app, log, ref):
            problems += _typed(ctx, app, ref, TYPED, TYPED)
            problems += placement_problems(presses(log), DVORAK_CODES)
            problems += [f"after typing: {item}" for item in
                         readback_problems(ime_settled(during, original), ime_settled(keymap_read(tier), original))]
        with focused_zoo(ctx, record=False) as zoo:
            problems += tab_moves_focus(ctx, zoo)
            problems += [f"after Tab: {item}" for item in
                         readback_problems(ime_settled(during, original), ime_settled(keymap_read(tier), original))]
        ctx.after("hyprland.keymap", keymap_read(tier))
    resync_input_method(ctx)
    if not raw.poll(lambda: keymap_read(tier) == original and keyboards(tier) == before, SETTLE_S):
        problems.append(f"the keymap did not come back: {readback_problems(original, keymap_read(tier))}")
    ctx.restored("hyprland.keymap", keymap_read(tier))
    require(not problems, "; ".join(problems))
