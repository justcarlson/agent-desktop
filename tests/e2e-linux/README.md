# Linux live end-to-end harness

## Run

## Tiers

## Legs

## KDE manual matrix

The Live Tiers above name GNOME only. A Plasma 5.24+ session is not hosted by the
runner, so the same surface is walked by hand on a real Plasma desktop and stays
outstanding until that walk is recorded. One row per cell; a cell that cannot run
on the session is written down with its reason, never left blank.

| Area | Steps | Expected shape | Record |
| --- | --- | --- | --- |
| Snapshot | `snapshot --app <app> -i` on a Plasma window | refs allocated, `complete: true` | pass / finding with the raw tree |
| Semantic click | `click <ref>` on a Plasma button | the button's own effect, read back from the app | pass / finding |
| Wheel and drag | `mouse-wheel` and `drag` over a Plasma window | pointer moves and the app reacts | pass / finding |
| Focus | `focus-window` on a covered Plasma window | `_NET_ACTIVE_WINDOW` names the target | outcome set, not a single value |
| Notifications | `list-notifications` after `notify-send` | the sender's summary appears once | pass / finding |
| Dismiss | `dismiss-notification 1 --expected-app <app>` | the banner is gone and no other banner moved | pass / finding |
| Tray | `snapshot --surface system-tray` with an SNI item registered | the item is listed and `click` reaches `Activate` | pass / finding |
| Menu surface | open a Plasma menu, `snapshot --surface menu` | item count equals the raw showing count | pass / finding |
| Wayland | repeat Snapshot, Semantic click and Focus under Plasma Wayland | focus refusals match the landed X11/Wayland decision | pass / finding |

## Reading a failure

## Reading a failure
