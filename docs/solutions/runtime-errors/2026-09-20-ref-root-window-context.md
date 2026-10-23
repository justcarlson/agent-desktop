---
module: agent-desktop-macos
date: 2026-09-20
problem_type: runtime_error
tags: [states, viewport, find, snapshot, consistency]
---

# Ref-rooted observations used the element as its own viewport

SF Symbols first-button `find` reported `offscreen` while `get --property states` on the same ref returned an empty
list. The button's bounds were x=194, y=82, width=0, height=0; the owning window was x=0, y=30, width=1496,
height=937.

Element-root observation initialized its build context with the ref's own stored bounds as the window bounds, so
hydrating the selected match clipped the zero-size button against itself. Live `get` read `AXWindow` instead. The
mismatch was in observation context, not a changed element or a stale ref.

## Repair

Element-root observation (`crates/macos/src/tree/query/mod.rs`) and live post-action state
(`crates/macos/src/actions/post_state.rs`) both compute the viewport with `ancestor_viewport` in
`crates/macos/src/tree/element_bounds.rs`: the owning `AXWindow` rectangle from `owning_window_bounds`, clipped by
every ancestor whose role clips its descendants (`clips_descendants`). There is no fallback to
`AXTopLevelUIElement`, because a window-less menu must not use the menu bar as its viewport. A failed `AXWindow` read
propagates as an error.

## Lesson

A ref-rooted read takes its viewport from the element's ancestors, never from the element's own bounds. Otherwise a
zero-size or freshly moved control reads `offscreen` in one command and visible in the next, and the two commands
disagree about the same ref.
