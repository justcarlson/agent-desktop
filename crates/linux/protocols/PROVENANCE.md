# Vendored Wayland protocols

`agent-desktop-linux` generates client bindings from these files with the
`wayland-scanner` proc macros at build time. The files are unmodified copies.

| File | Upstream | Revision | Upstream path | SHA-256 |
|---|---|---|---|---|
| `hyprland-toplevel-mapping-v1.xml` | https://github.com/hyprwm/hyprland-protocols | `v0.8.0` (`044a434ec371f985655389bcef09286b1af3d77c`) | `protocols/hyprland-toplevel-mapping-v1.xml` | `05464521a13c96ed274a8545d84ac03980d94b32bd55a8467c7949ed1a1ee96e` |
| `LICENSE.hyprland-protocols` | https://github.com/hyprwm/hyprland-protocols | `v0.8.0` (`044a434ec371f985655389bcef09286b1af3d77c`) | `LICENSE` | `ea36a60a50dfa150ca3c5816cb6b0ad5b559da380e9f5b2babd4abbf7962a35a` |

Both files are under the BSD 3-Clause License. The protocol file carries its
own copyright notice; the repository license covers the rest of
hyprland-protocols. `THIRD_PARTY_NOTICES.md` at the repository root
reproduces both, and the release archives and the npm package ship it.

To update, copy both files from a tagged upstream revision, then update the
revision and the hashes in this table and in `THIRD_PARTY_NOTICES.md`.
`scripts/check-vendored-protocols.sh` fails when a hash here does not match
the file.
