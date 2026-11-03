"""Prove scoped terminal key delivery with raw child byte logs."""
from __future__ import annotations

import contextlib
from pathlib import Path

from oracle import atspi_raw as raw, proc
from runner import attached_session
from runner.apps import OwnedApp
from runner.helper import require
from scenarios.hyprland_common import bystander, desktop, main_instance
from scenarios.hyprland_focus_controls import (
    _new_client, _register, _stop, client, clients, cursor, listed_ids, move, restore_desktop,
)

SCRIPT = Path(__file__).resolve().parents[1] / 'apps' / 'terminal_reader.py'


def problems(result: dict, reader: bytes, other: bytes) -> list[str]:
    found = []
    if not result.get('ok'):
        found.append(f'press refused: {result}')
    proof = ((result.get('data') or {}).get('details') or {}).get('focus_proof')
    if proof != 'compositor_only':
        found.append(f'focus proof is {proof!r}')
    if reader != b'a\r':
        found.append(f'reader bytes are {reader!r}')
    if other:
        found.append(f'bystander received {other!r}')
    return found


def type_problems(result: dict, before: bytes, after: bytes) -> list[str]:
    error = result.get('error') or {}
    expected = {'delivery': 'not_delivered', 'retry': 'safe'}
    found = []
    if result.get('ok') or error.get('code') not in ('APP_NOT_FOUND', 'INVALID_ARGS') or error.get('disposition') != expected:
        found.append(f'type did not refuse safely: {result}')
    if after != before:
        found.append(f'type changed reader bytes: {before!r} -> {after!r}')
    return found


def type_argv(window_id: str) -> list[str]:
    return ['type', window_id, 'x']


def envelope(result) -> dict:
    return {'ok': result.ok, 'data': result.data, 'error': result.error}


@contextlib.contextmanager
def reader(tier, name: str):
    log, ready = (tier.runtime_dir / f'{name}.{suffix}' for suffix in ('bytes', 'ready'))
    for path in (log, ready):
        path.unlink(missing_ok=True)
    known = {item['address'] for item in clients(tier)}
    argv = ['foot', '--app-id', name, '--title', name, 'python3', '-I', str(SCRIPT), str(log), str(ready)]
    attached_session.spawn(tier, argv, dict(tier.env), tier.runtime_dir / f'{name}.log')
    found = raw.poll(lambda: _new_client(tier, name, known), 10.0)
    require(found, f'{name} did not map')
    start = proc.stat_fields((Path('/proc') / str(found['pid']) / 'stat').read_text())[0]
    owned = _register(tier, OwnedApp(name, found['pid'], start, [], [], ''))
    try:
        require(raw.poll(ready.exists, 5.0), f'{name} raw reader was not ready')
        yield owned, found, log
    finally:
        _stop(tier, owned)


def press_window_id_compositor_only(ctx) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    holder, point = bystander(tier), cursor(tier)
    ctx.before('hyprland.desktop', desktop(tier, main))
    try:
        with reader(tier, 'ad-e2e-reader') as (app, target, log), reader(tier, 'ad-e2e-key-bystander') as (_, other, other_log):
            here = client(tier, holder['address'])['workspace']['name']
            move(tier, target['address'], here)
            move(tier, other['address'], here)
            restore_desktop(tier, other, point)
            ids = listed_ids(ctx.product(['list-windows']), app.pid)
            require(len(ids) == 1 and ids[0].startswith('w-hypr-'), f'not a compositor-only window: {ids}')
            errors = []
            for key, expected in [('a', b'a'), ('Return', b'a\r')]:
                result = ctx.product(['press', key, '--window-id', ids[0]])
                require(result.ok, f'press {key} refused: {result.raw}')
                require(raw.poll(lambda: log.read_bytes() == expected, 5.0), f'{key}: reader bytes {log.read_bytes()!r}')
                if ((result.data or {}).get('details') or {}).get('focus_proof') != 'compositor_only':
                    errors.append(f'{key}: missing compositor_only proof')
            errors += problems(envelope(result), log.read_bytes(), other_log.read_bytes())
            before = log.read_bytes()
            refused = ctx.product(type_argv(ids[0]))
            errors += type_problems(envelope(refused), before, log.read_bytes())
            require(not other_log.read_bytes(), 'bystander received input')
            ctx.after('hyprland.desktop', desktop(tier, main))
            require(not errors, '; '.join(errors))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored('hyprland.desktop', desktop(tier, main))
