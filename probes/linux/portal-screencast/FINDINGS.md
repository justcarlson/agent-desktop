# Portal ScreenCast as the R7 capture backend on Hyprland

Verdict: **stop**. On stock xdg-desktop-portal-hyprland (xdph) 1.4.1, the ScreenCast portal cannot meet R7.
Q1 (consent) and Q2 (window identity) are not feasible. Q3 (hidden workspace) and Q4 (freshness) are feasible
with conditions, but those conditions do not matter while Q1 and Q2 fail.

R7 asks for four things. `screenshot --window-id` captures exactly the bound toplevel, including on a hidden
workspace. It succeeds only for a frame the compositor produced after the request, within a bounded deadline. A
permission denial fails closed and never opens a dialog. One backend ships first, with no silent fallback.

Every answer here comes from reading source. Nothing in this directory has run. The scripts are for a throwaway
VM and refuse to start unless `PORTAL_PROBE_VM=1` is set and `systemd-detect-virt --vm` succeeds. No portal,
PipeWire or D-Bus capture method was called on the host.

## Sources

The sources were cloned into a fresh `/tmp` directory as data. None of them was built or run.

| id | source | version |
| --- | --- | --- |
| `xdph` | github.com/hyprwm/xdg-desktop-portal-hyprland | tag `v1.4.1`, `VERSION` 1.4.1; host package `xdg-desktop-portal-hyprland 1.4.1-2` |
| `xdp` | github.com/flatpak/xdg-desktop-portal | tag `1.22.1`; host package `xdg-desktop-portal 1.22.1-2` |
| `hl` | github.com/hyprwm/Hyprland | tag `v0.56.2` (efb5099); host package `hyprland 0.56.2-4` |
| `pw` | gitlab.freedesktop.org/pipewire/pipewire | tag `1.6.9`; host package `pipewire 1:1.6.9-1` |
| `spa` | `/usr/include/spa-0.2/spa/buffer/meta.h` | installed with pipewire 1.6.9 |

The xdph package ships `/usr/bin/hyprland-share-picker`, the portal binary, a user unit, a D-Bus service file and
`hyprland.portal`, and no config documentation. The config keys and defaults are in
`xdph src/core/PortalManager.cpp:74-79`: `screencopy:max_fps` 120, `allow_token_by_default` 0,
`custom_picker_binary` "", `force_shm` 0, `cursor_mode` 0.

## Answers

| id | question | verdict | answer |
| --- | --- | --- | --- |
| PC-1 | Consent without a dialog | **not feasible** | The first run always opens the picker. A missing, used or revoked token opens it again. The API has no fail-closed mode. |
| PC-2 | Name one toplevel with no picker | **not feasible** | The request cannot name a window. A restore falls back silently to the first window of the stored class. The stream does not say which window it carries. |
| PC-3 | Frames from a hidden workspace | feasible with conditions | The window source uses the same Hyprland renderer as `ext_image_copy_capture`, which M0 probe 4 measured on a hidden workspace. |
| PC-4 | Prove a buffer is fresh | feasible with conditions | The header `pts` is Hyprland's `CLOCK_MONOTONIC` copy time, so a consumer can compare it with its request time. Damage carries no signal. |

### PC-1 Consent

- **The first run always opens a dialog.** Without valid restore data, `SelectSources` runs the picker
  (`xdph src/portals/Screencopy.cpp:225-228`). That is `hyprland-share-picker`, or the program in
  `custom_picker_binary` (`xdph src/shared/ScreencopyShared.cpp:57`). If the picker prints no `[SELECTION]`, the
  selection stays `TYPE_INVALID` (`xdph src/shared/ScreencopyShared.hpp:24,33-34`, `ScreencopyShared.cpp:64-80`)
  and `SelectSources` returns 1 (`Screencopy.cpp:251`).
- **`allow_token_by_default` only pre-ticks a checkbox.** It adds `--allow-token` to the picker command
  (`ScreencopyShared.cpp:49-55`). The picker uses that flag only to check its "allow restore token" box
  (`xdph hyprland-share-picker/main.cpp:83-86,106-107`). The user still has to click a source. The `r` flag is
  sent with that click (`main.cpp:133-134`, `ScreencopyShared.cpp:86-94`).
- **xdph ignores `persist_mode`.** It stores the value and never reads it (`Screencopy.cpp:188-190`). When the
  picker returned `r`, xdph always answers `persist_mode` 2 with restore data (`Screencopy.cpp:274-278`). The
  frontend keeps the smaller of the requested and returned modes (`xdp src/xdp-session-persistence.c:362-365`).
- **A token works once.** The frontend looks up the token, deletes it and passes the stored data to xdph
  (`xdp src/xdp-session-persistence.c:212-255`). It saves a new token when the session closes
  (`xdp src/screen-cast.c:130-141`). The spec says "The restore token is invalidated after using it once"
  (`xdp data/org.freedesktop.portal.ScreenCast.xml:120`). A CLI would have to store a rotating secret, and one
  lost write puts the dialog back.
- **A revoked or unknown token opens the dialog.** The spec: "If the stored session cannot be restored, this
  value is ignored and the user will be prompted normally" (`ScreenCast.xml:115-118`). xdph sends missing restore
  data to the picker (`Screencopy.cpp:197-201,225-228`). No `SelectSources` option asks to fail instead of
  prompting (`ScreenCast.xml:87-146`), and xdph logs any other key as unused (`Screencopy.cpp:191-192`). R7
  requires a denial that fails closed and opens no dialog.
- **Each new target window needs a new grant.** Restore data names one window (`Screencopy.cpp:17-39`).
  `screenshot --window-id` targets any bound window, so the first capture of each new window opens the picker.
- **The compositor permission covers xdph, not agent-desktop.** Hyprland checks the `screencopy` permission of
  the client that asked for the frame (`hl src/managers/screenshare/ScreenshareFrame.cpp:154,360`). On this path
  that client is xdph (`hl src/protocols/ToplevelExport.cpp:36`). A deny rule draws the "denied" texture and
  delivers it as a successful copy (`ScreenshareFrame.cpp:369-374`). A pending rule opens Hyprland's own
  permission popup (`hl src/managers/permissions/DynamicPermissionManager.cpp:146-148,226-262`). An
  `ecosystem:enforce_permissions` rule therefore cannot target agent-desktop on this path, and a rule on xdph
  yields an image, not an error.
- **`custom_picker_binary` does not rescue R7.** A program with no UI could print
  `[SELECTION]r/window:<handle>`. But the setting is global: xdph runs that program for every app that asks for a
  ScreenCast. Its environment holds only `WAYLAND_DISPLAY`, `QT_QPA_PLATFORM`, `XCURSOR_SIZE`,
  `HYPRLAND_INSTANCE_SIGNATURE` and `XDPH_WINDOW_SHARING_LIST` (`ScreencopyShared.cpp:58-62`). Nothing names the
  requesting app, so the program cannot grant agent-desktop and still ask the user about a browser. Using it also
  means editing the owner's `xdph.conf` and removing the portal's consent UI, which is the reason to choose the
  portal at all.
- Box fact: this host's `~/.config/hypr/xdph.conf` sets `allow_token_by_default = true` and
  `custom_picker_binary = hyprland-preview-share-picker` (package `hyprland-preview-share-picker 0.2.1-3`). That
  program is another interactive picker, so the answer is the same.

### PC-2 Window identity

- **The request cannot name a window.** `SelectSources` takes `types`, `multiple`, `cursor_mode`,
  `restore_token` and `persist_mode` (`ScreenCast.xml:87-146`). xdph ignores every other key
  (`Screencopy.cpp:108-194`). The window comes from the picker or from restore data. The app holds only an opaque
  token that the frontend swaps for stored data (`xdp src/xdp-session-persistence.c:212-255`), so it cannot write
  restore data for a window it chooses.
- **xdph's window identity lives only as long as one xdph binding.** Each picker entry has the lower 32 bits of
  xdph's own `zwlr_foreign_toplevel_handle_v1` proxy pointer, the class, the title and the Hyprland address
  (`ScreencopyShared.cpp:28-40`). A selection matches on those 32 bits (`ScreencopyShared.cpp:101-110`,
  `xdph src/shared/ToplevelManager.cpp:87-94`). The address is 0 until the async `hyprland_toplevel_mapping`
  reply arrives (`xdph src/shared/ToplevelMappingManager.cpp:9-39`). xdph has no pid, process start time or
  `ext_foreign_toplevel` identifier. It tracks toplevels through the wlr protocol only
  (`ToplevelManager.cpp:40-50`).
- **A restore falls back silently to another window.** Restore data stores the full proxy pointer and the class
  (`Screencopy.cpp:27-28`). It counts as valid if any window of that class exists (`Screencopy.cpp:197-201`).
  xdph uses the stored pointer if it still matches, else the first window of the class
  (`Screencopy.cpp:208-214`, `ToplevelManager.cpp:78-85,96-103`). The pointer stops matching after xdph
  restarts, or after the last session closes and the toplevel manager unbinds (`ToplevelManager.cpp:62-75`,
  called from `Screencopy.cpp:64`). With two windows of one class, a restore can capture the wrong one and
  report success. R7 forbids both the wrong window and the silent fallback.
- **The response does not identify the window.** A window stream carries `position` (always 0,0), `size`,
  `source_type` and `pipewire-serial`. Only output streams get `mapping_id` (`Screencopy.cpp:294-303`). The
  consumer can check the window only by guessing from its size.
- **How xdph maps a window source.** Picker handle, then `zwlr_foreign_toplevel_handle_v1`, then
  `hyprland_toplevel_export_manager_v1.capture_toplevel_with_wlr_toplevel_handle` (`Screencopy.cpp:375-381`),
  then Hyprland's `windowFromHandleResource` (`hl src/protocols/ToplevelExport.cpp:24`). xdph does not use
  `ext_image_copy_capture` for windows.

### PC-3 Hidden workspace

- **The capture path.** `captureToplevel` opens a window screenshare session keyed by xdph's client and the
  window (`hl src/protocols/ToplevelExport.cpp:30-36`). Each frame calls `renderWindow`, which renders that window
  alone, standalone and with its position ignored (`hl src/managers/screenshare/ScreenshareFrame.cpp:316-332`;
  parameter names in `hl src/render/Renderer.hpp:268`). This is the same `CScreenshareSession` and
  `CScreenshareFrame` code that `ext_image_copy_capture` uses. M0 probe 4 measured that code on a hidden special
  workspace: frames arrived in 2 to 15 ms and the counter pixels changed (agent-desktop-hyprland
  `probes/out/p4.log`).
- **What it renders.** The window's own surfaces, with nothing behind them. A window with the `noscreenshare`
  rule yields the "denied" texture (`ScreenshareFrame.cpp:366-374`). With cursor overlay on, the cursor is drawn
  only when the window has focus and the pointer is over it (`ScreenshareFrame.cpp:334-351`).
- **Condition: later frames wait for the monitor.** A pending frame is copied only when the window's monitor
  commits and the window geometry intersects that monitor (`hl src/managers/screenshare/ScreenshareManager.cpp:14-45`).
  A session's first frame schedules a monitor frame (`ScreenshareFrame.cpp:124-129`). Later frames wait for
  something else to commit on that monitor, because xdph copies with `ignore_damage` false
  (`xdph Screencopy.cpp:619`) and Hyprland forces monitor damage only when it is true
  (`hl ToplevelExport.cpp:128-129`). After 500 ms with no frame the session stops sharing
  (`hl src/managers/screenshare/ScreenshareSession.cpp:72-78`), and the next frame counts as a first frame again
  (`ScreenshareSession.cpp:170-171`). On an idle output a frame can wait about 500 ms.
- **Condition: the window must stay mapped.** Unmapping it stops the session (`ScreenshareSession.cpp:22`,
  `ScreenshareFrame.cpp:41-70`).

### PC-4 Freshness

- **The pts is the copy time.** On `RESULT_TIMESTAMP`, Hyprland stores `Time::steadyNow()`
  (`hl src/protocols/ToplevelExport.cpp:150`), which is `std::chrono::steady_clock`
  (`hl src/helpers/time/Time.cpp:35-36`), `CLOCK_MONOTONIC` on Linux. The result fires when the copy is
  committed, not at page flip (`ScreenshareFrame.cpp:149-151`, where an upstream TODO asks whether that is
  right). It reaches xdph in `ready` (`ToplevelExport.cpp:140-143`). xdph writes it to `spa_meta_header.pts` and
  increments `seq` (`xdph Screencopy.cpp:523-534,1192-1196`). `pts` is in nanoseconds (`spa meta.h:77-89`). A
  consumer that reads `CLOCK_MONOTONIC` before `Start` can require `pts` after that time and a frame before its
  deadline. The header exists only if the stream negotiates `SPA_META_Header` (`xdph Screencopy.cpp:1192-1193`).
- **Reject corrupted buffers.** A buffer queued without a ready frame carries `SPA_META_HEADER_FLAG_CORRUPTED`
  and `SPA_CHUNK_FLAG_CORRUPTED` (`xdph Screencopy.cpp:1186-1195,1241`).
- **Through GStreamer.** `pipewiresrc` copies `pts` to `GST_BUFFER_PTS` and `seq` to the buffer offset
  (`pw src/gst/gstpipewiresrc.c:755-770`), and maps the chunk corrupted flag (`gstpipewiresrc.c:866-868`).
  Keepalive buffers rewrite the pts (`gstpipewiresrc.c:1640-1670`), so the probe sets `keepalive-time=0`.
- **Damage carries no signal.** Hyprland reports the whole buffer as damaged on every frame
  (`hl ScreenshareFrame.cpp:131-140`). xdph only ever increments `damageCount`, and after four rects it overwrites
  `damage[0]` with the full frame (`xdph Screencopy.cpp:439-444,554-559`).
- **Caveat: old pixels with a new pts.** While Hyprland's permission for xdph is pending, it saves a snapshot
  (`hl ScreenshareFrame.cpp:153-160`). The first copy after approval draws that snapshot
  (`ScreenshareFrame.cpp:377-381`) with a pts from copy time. Changing content, such as probe 4's ticking
  counter, proves freshness in that case; the pts alone does not.

## What a VM run must measure

`run-scenarios.sh` runs these checks in a throwaway VM. It swaps the picker for `stub-picker.sh`, which draws
nothing, logs each call and answers "deny" or a selection by Hyprland address. Each logged call is one place where
stock xdph would have opened a dialog. `portal_screencast_probe.py` writes one JSON record per run.

| row | scenario label | measures | expected from source |
| --- | --- | --- | --- |
| PC-1 | `q1-first-run-deny` | picker calls on a first run with no token | 1 call; `select_code` not 0 |
| PC-1 | `q1-restore-same-xdph` | picker calls when restoring a fresh token | 0 calls; `restore_token_out` differs from the input |
| PC-1 | `q1-restore-after-revoke` | picker calls after `PermissionStore.Delete("screencast", token)` | 1 call, which is a dialog with the real picker |
| PC-1 | manual | `permission = <xdph path>, screencopy, deny` with `ecosystem:enforce_permissions` | a successful frame showing the denied image, not an error |
| PC-2 | `q2-grant-b` | whether `XDPH_WINDOW_SHARING_LIST` has a nonzero address when the picker runs | nonzero unless the mapping reply is late |
| PC-2 | `q2-restore-after-xdph-restart` | stream size after a restore across an xdph restart: 600x400 is A, 900x700 is B | A if A is first in xdph's list: a silent wrong window |
| PC-3 | `q3-hidden-ticking` | frames from B on `special:adprobe`, hash changes, first-frame latency | frames arrive, hashes differ, latency near probe 4 |
| PC-3, PC-4 | `q4-hidden-idle-output` | gaps between frames with animations off and B hidden | gaps up to about 500 ms |
| PC-4 | every run with frames | `pts_after_request`, `corrupted`, `seq` | true, false, increasing |

No result here can lift the stop. PC-1 and PC-2 follow from the portal API and xdph's restore logic, and that
logic does not depend on timing.
