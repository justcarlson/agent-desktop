# XKB keymap test fixtures

The two `.xkb` files are compiled XKB keymaps. Only tests read them, through
`include_str!` in `*_tests.rs` files, so no build or release ships them.

| File | Command | SHA-256 |
|---|---|---|
| `us_ru.xkb` | `xkbcli compile-keymap --rules evdev --model pc105 --layout us,ru --options grp:alt_shift_toggle` | `0a2f89b71de32b710b8a92a45230ba0b64d15be977d18a13b1ec97262b86a27a` |
| `de_nodeadkeys_compose_ralt.xkb` | `xkbcli compile-keymap --rules evdev --model pc105 --layout de --variant nodeadkeys --options compose:ralt` | `298937d9582bcdbc0c12e5bdc8f009e5720f6207b2a41064375dadee414dbe9a` |

- Data: xkeyboard-config 2.48 (https://gitlab.freedesktop.org/xkeyboard-config/xkeyboard-config).
- Compiler: `xkbcli` from libxkbcommon 1.13.2.

The keymaps are built from xkeyboard-config data. Its `COPYING` file holds
the copyright and permission notices, which ask that the notices travel with
copies. `LICENSE.xkeyboard-config` beside this file is that `COPYING` from
release 2.48, unmodified (SHA-256
`fc03f6535d16b3fe9beece45b84214a37e6ca4b6550c03cadaed7e1b951fb294`).

To update a fixture, run its command with the same versions, then update its
hash here. Other xkeyboard-config or libxkbcommon versions can give a
different file.
