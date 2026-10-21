# Frozen surface verification

Surface-guide checks used the unchanged binary
`~/ad-wt/bin-3.14-8295f30b/agent-desktop`, built from
`8295f30b4a2b871e1bb5f9fceb5357759790b691`.
SHA-256: `614d648348a9876c12298b77d7218f25a270a51973ba5d9b6dfe79fa2f93de12`.
These are private-session measurements on GNOME Shell 46, not live-desktop
consent tests. Existing capture, clipboard, lifecycle and sandbox guidance was
outside this surface pass.

The box retains the scripts, verbatim commands, stdout, stderr and return codes
under `~/ad-wt/skill-pass-c/`. Each JSON file is an ordered receipt array;
`label` identifies a command or an independent observation. Commands ran in
`bash`, with the frozen binary first on PATH. Private sessions used
`scripts/private-session.sh --wm --` for Openbox, that session wrapper plus
`scripts/private-mutter-shell.sh` for Mutter X11, and the session wrapper plus
`scripts/private-wayland-shell.sh` for Wayland. No live app or clipboard was
used. PipeWire and test services belonged to those private sessions.

The table counts **57 behavioural claims**, including successful reads/actions
and expected refusals. O = Openbox X11, M = Mutter X11, W = Wayland. A backend
listed here identifies a measurement, not a claim that every toolkit was run on
every backend. Native AT-SPI state, D-Bus service ownership/signals, fixture
receiver logs and X11 window inventory supplied independent observations.

| # | Claim and measured directions | Backend; receipt / labels |
|---|-------------------------------|--------------------------|
| 1 | App status advertises window/sheet/menu; shell availability differs | O/M/W; `openbox`, `mutter`, `wayland`: `status` |
| 2 | GTK4 opener and inventory include `type: menu`, count 10; closed inventory omits it | O/M/W; `openbox-menu`, `focus`, `wayland-surprises`: menu example/tree and after inventory |
| 3 | Open menu can be read; closed menu snapshot is `ELEMENT_NOT_FOUND` | O/W; `openbox-menu`, `wayland-surprises`: closed/menu reads |
| 4 | Menu closure invalidates its ref with `STALE_REF`, `menu_closed` | O; `openbox-menu`: `closed-menu-ref`; `toolkits-rest`: `qt-stale` |
| 5 | GTK4 raw unnamed items have readable derived snapshot labels | O/M/W; `about`, `focus`, `wayland-menu`: menu tree and native items |
| 6 | Readable GTK4 menu snapshot does not ensure live `find`; identity refusal | O/M; `about`, `focus`: `menu-find` |
| 7 | GTK3/GTK4/Qt item picks fail identity resolution before delivery; no item effect | O/M; `toolkits`, `about`, `focus`, `toolkits-rest`: item pick and receiver/native observations |
| 8 | Qt line-edit context popup exists natively but is absent from menu inventory | O; `toolkits-rest`: Qt context inventory/snapshot/native windows |
| 9 | GTK3 menu bar alone is closed; File opener exposes menu | O; `toolkits`: idle inventory, File opener and menu tree |
| 10 | GTK3 context menu is a separate accessible popup | O; `toolkits-rest`: GTK context opener, surfaces, tree and native windows |
| 11 | Qt File popup exposes two entries; Escape removes the menu surface | O; `toolkits-rest`: Qt File/read/close |
| 12 | Chromium DOM menuitem does not become an app menu surface | O; `toolkits-rest`: Chromium DOM inventory and native nodes |
| 13 | Chromium native context menu appears in opener surfaces and menu snapshot, then closes | O; `toolkits-rest`: Chromium context/read/closed wait |
| 14 | No GNOME tree: shell snapshot validation refuses; headed opener reports `APP_NOT_FOUND` | O; `openbox`: `no-shell-snapshot`, `no-shell-open` |
| 15 | GNOME adds the three shell surfaces absent on Openbox | M/W; `mutter`, `wayland`: `status` |
| 16 | Shell container reads need no app window; ordinary shell window inventory is empty | M/W; `mutter`, `wayland`: shell snapshots, `shell-list-windows` |
| 17 | Hidden system menu: headless open denied, headed open makes native container visible | M/W; `mutter`, `wayland`: `shell-headless-open`, `system-before/after` |
| 18 | Headless snapshot reads a closed system menu without opening it | M/W; `mutter`, `wayland`: `shell-escape`, `shell-headless-snapshot` |
| 19 | Headed action-center opener makes message list visible; Escape closes it | M/W; `mutter`, `wayland`: `message-before/after`, open/read/escape |
| 20 | Taskbar is refused even with enabled Ubuntu Dock | M/W; `tray`, `wayland-tray`: extensions and dock refusals |
| 21 | Overview is not a snapshot-surface argument (`INVALID_ARGS`) | M/W; `tray`, `wayland-tray`: `overview-not-surface` |
| 22 | Overview blocks X11 focus; Escape followed by headed focus makes Calculator active | M; `focus`: overview refusal, recovery, native ACTIVE |
| 23 | Ordinary Wayland pointer refuses before delivery; native focus-window also refuses | W; `wayland-menu`, `wayland-surprises`: pointer refusal, focus setup |
| 24 | Notification popup click needs a stream: absent PipeWire refuses; private PipeWire enables dismissal | W; `wayland`: `missing-stream-dismiss`, `notification-example` |
| 25 | No daemon is a refusal, not an empty list (also notification wait) | O; `openbox`: `no-daemon`, `no-daemon-wait` |
| 26 | Non-GNOME notification server is explicitly unsupported | O; `openbox`: `unsupported-daemon` |
| 27 | Unreadable notification service returns retryable `TIMEOUT` | O; `openbox`: `unreadable-daemon` |
| 28 | GNOME server identity permits notification reading | M/W; `mutter`, `wayland`: `notification-server`, list |
| 29 | Empty list and headed clear return zero; headless empty clear is denied | M/W; `mutter`, `wayland`: empty list/clear and headless clear |
| 30 | Hidden message-list reading returns posted entries without banner duplication | M/W; `mutter`, `wayland`: sender IDs, notification list |
| 31 | App/text filtering preserves original 1-based indices | M/W; `mutter`, `wayland`: list and `notification-filter` |
| 32 | Text searches body and app name; an absent needle yields no entries | W; `wayland`: body/app/no-match filters |
| 33 | Existing notification does not satisfy wait; a new entry after observed baseline does | M/W; `mutter`, `wayland`: `wait-existing`, baseline observation, `wait-new` |
| 34 | Single mutation without expected identity is `INVALID_ARGS` | M/W; `mutter`, `wayland`: `missing-identity` |
| 35 | Headless single dismissal is `POLICY_DENIED` | M/W; `mutter`, `wayland`: `headless-dismiss` |
| 36 | Wrong title or missing index is `NOTIFICATION_NOT_FOUND` | M/W; `mutter`, `wayland`: `wrong-title`, `missing-notification` |
| 37 | Unlisted action is `ACTION_NOT_SUPPORTED`, with available actions | M/W; `mutter`, `wayland`: `missing-action` |
| 38 | Guarded dismissal removes Beta and preserves bystanders; X11 may error after delivery | M/W; `mutter`, `notification-recheck`, `wayland`: notification example and independent target/bystander observation |
| 39 | Filtered clear can stop after one delivered click on X11; Wayland clears its matching set | M/W; `notification-recheck`, `wayland`: filtered clear and subsequent state |
| 40 | Action pick delivers unverified; evidence differs (`settle_elapsed` / `action_invoked`) | M/W; `mutter`, `wayland`: action reply and independent Alpha removal |
| 41 | Unfiltered clear removes captured list entries; an adopted message list remains open | W; `wayland-surprises`: before/after list, closed-ID signals, adopted state |
| 42 | Global Escape is not proof of app-menu closure; one Wayland timeout, isolated recheck closes | W; `wayland-menu`, `wayland-surprises`: complete menu example |
| 43 | Watcher absent refuses despite advertised tray; present watcher permits snapshot | M/W; `mutter`: missing watcher; `tray`, `wayland-tray`: populated snapshots |
| 44 | Unreadable watcher returns retryable `TIMEOUT` | W; `wayland-tray`: `unreadable-watcher` |
| 45 | Enabled AppIndicator provides watcher owned by private Shell; stock session lacks it | M/W; `mutter`, `tray`, `wayland-tray`: missing watcher, owner PID and extension state |
| 46 | SNI Activate and ContextMenu reach corresponding receiver methods headlessly | M/W; `tray`, `wayland-tray`: method calls and receiver log |
| 47 | Methodless Indicator is headless-denied; headed fallback works on X11, refuses on Wayland | M/W; `tray`, `wayland-tray`: Indicator clicks and receiver log |
| 48 | Virtual leaf pick sends dbusmenu Event clicked, not a synthetic SNI primary method | M/W; `tray`, `wayland-tray`: `tray-leaf`, receiver log |
| 49 | Check toggles; identical old leaf remains usable after revision changes | M/W; `tray`, `wayland-tray`: check, old-ref-new-revision and receiver |
| 50 | Submenu pick invokes AboutToShow, not leaf Event | M/W; `tray`, `wayland-tray`: submenu and receiver log |
| 51 | Disabled entry sends no event in either mode; error gate differs on Wayland | M/W; `tray`, `wayland-tray`: disabled clicks and no-receiver observation |
| 52 | Passive transition retains item with hidden state | M/W; `tray`, `wayland-tray`: Passive receiver signal and snapshot |
| 53 | Owner departure makes a previously working tray ref stale before delivery | M/W; `tray`, `wayland-tray`: initial click, `tray-owner-gone` |
| 54 | Expand on tray item is unsupported | M/W; `tray`, `wayland-tray`: `tray-unsupported` |
| 55 | Default virtual getter fails `invalid_native_handle`; paired bounds/visible reads work | M/W; `tray`, `wayland-tray`: get; Wayland bounds and visible |
| 56 | Tray find returns item and count with the same scoped identity | W; `wayland-tray`: find and find-count |
| 57 | Tray virtual menu is not an ordinary shell app-menu surface | M/W; `tray`, `wayland-tray`: `tray-popup-is-not-app-menu` |

All three surface-reference shell blocks were executed verbatim in bash.
The Calculator example ran on O/M/W; the shell and guarded notification
examples ran on M/W, with their missing prerequisites measured on O. The shell
example's literal command lines were executed in sequence with independent
container checks between them. All Linux-skill shell blocks also pass `bash -n`.
`mutter-example` first ran with Overview covering Calculator and did not open
the menu. `mutter-example-focus` rechecked the documented Escape/focus recovery,
observed Calculator ACTIVE, then ran the same block with every JSON result
successful and independently observed the menu closed afterwards.

Surprises were retained and rechecked once, never silently replaced: the first
Calculator selector matched two Main Menu nodes, so the example now selects the
`switch`; menu item actionability failed despite readable labels; X11 dismissal
could remove a target before returning a partial-read error; Wayland Escape
closed only on the isolated recheck. A whole-tree “all title nodes vanished”
oracle was too broad for clear-all: the recheck observed both notification IDs'
close signals and an empty message list while one hidden label remained.
Harness-only aborts after these retained results do not count as product
failures: the Qt/GTK4 helper initially expected an item pick to succeed, and the
Chromium stale-ref helper expected a label without its shortcut suffix. Closure
and stale-ref behaviour have separate successful receipts.

The frozen binary predates Linux skill embedding: `skills get platform` returns
`INVALID_ARGS`, “This build's operating system has no platform skill yet”.
The current documentation branch's Linux selection arm and embedded reference
list were checked in source; this pass does not claim that frozen binary serves
the edited files. Menu/notification/tray errors are documented outcomes of this
binary, not product fixes in this documentation commit.
