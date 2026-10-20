from __future__ import annotations

import re

from oracle.atspi_raw import call, connect
from oracle.gsettings import tool
from runner.session import Tier


def window_ids(text: str) -> list[str]:
    if "=" not in text and "#" not in text:
        return []
    body = re.split(r"[=#]", text, maxsplit=1)[1]
    return [hex(int(value, 16)) for value in re.findall(r"0x[0-9a-fA-F]+", body)
            if int(value, 16)]


def active_window(tier: Tier) -> str | None:
    ids = window_ids(tool(tier, ["xprop", "-root", "_NET_ACTIVE_WINDOW"]))
    return ids[0] if ids else None


def stacking(tier: Tier) -> list[str]:
    return window_ids(tool(tier, ["xprop", "-root", "_NET_CLIENT_LIST_STACKING"]))


def parse_geometry(text: str) -> tuple[int, int, int, int]:
    fields = ["Absolute upper-left X", "Absolute upper-left Y", "Width", "Height"]
    values = []
    for field in fields:
        match = re.search(rf"^\s*{field}:\s*(-?\d+)\s*$", text, re.MULTILINE)
        if match is None:
            raise ValueError(f"xwininfo omitted {field}")
        values.append(int(match[1]))
    if min(values[2:]) <= 0:
        raise ValueError("invalid window dimensions")
    return tuple(values)


def geometry(tier: Tier, window: str) -> tuple[int, int, int, int]:
    return parse_geometry(tool(tier, ["xwininfo", "-id", window]))


def client_origin(tier: Tier, window: str) -> tuple[int, int]:
    return geometry(tier, window)[:2]


def parse_pointer(text: str) -> tuple[int, int]:
    fields = dict(re.findall(r"^(X|Y)=(-?\d+)$", text, re.MULTILINE))
    return int(fields["X"]), int(fields["Y"])


def pointer(tier: Tier) -> tuple[int, int]:
    return parse_pointer(tool(tier, ["xdotool", "getmouselocation", "--shell"]))


def set_above(tier: Tier, window: str, on: bool) -> None:
    tool(tier, ["wmctrl", "-ir", window, "-b", f"{'add' if on else 'remove'},above"])


def current_desktop(tier: Tier) -> int | None:
    text = tool(tier, ["xprop", "-root", "_NET_CURRENT_DESKTOP"])
    match = re.search(r"=\s*(\d+)\s*$", text)
    return int(match[1]) if match else None


def overview_active(tier: Tier) -> bool:
    return call(connect(tier.bus_address), "org.gnome.Shell", "/org/gnome/Shell",
                "org.freedesktop.DBus.Properties", "Get", "(ss)",
                ("org.gnome.Shell", "OverviewActive"))[0]
