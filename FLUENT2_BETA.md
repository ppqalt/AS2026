# AS2026 — Fluent 2 / Dark Spruce beta

This branch is a **visual beta**, deliberately separate from stable `main`.
It starts at AS2026 v0.5.7 and preserves the calculation, storage, Emit,
import/export, updater, and competition model without modification.

## Design system

Slint 1.16 already renders its standard `Button`, `LineEdit`,
`ComboBox`, and `ScrollView` with the **Fluent style** selected explicitly
in `build.rs`. Keep these native Fluent widgets rather than duplicating
controls just for appearance.

`ui/theme.slint` is the only source for UI color and spacing tokens.
Dark Spruce is close to charcoal with a very restrained forest tint:

| Token | Hex | Usage |
|---|---|---|
| canvas | `#0D1210` | page |
| surface | `#151C19` | ordinary panels |
| surface-raised | `#1C2722` | quiet raised elements |
| line | `#2C3931` | one-pixel separators |
| accent-fill | `#356F54` | primary actions and selection rail |
| accent-soft | `#233B2D` | selected rows and navigation |
| ink | `#E8EEE9` | main text |
| muted | `#A3B0A7` | secondary labels |

Spacing follows Fluent 2's four-pixel ramp. Default controls are 36 px high,
list rows 38 px, and corners are 5–6 px. Dark Spruce green is reserved
for action, selection, and success states; not every border or title.

Google Sans is the requested preferred font. The application asks the OS for
`Google Sans` through `Window.default-font-family`; it **does not ship or
download any font files**, and Slint/system font substitution applies when
Google Sans is unavailable.

## Behavior and performance

- No app-authored animation, transition, blur, acrylic, drop shadow,
  gradient, glass effect, or semi-transparent modal overlay.
- Hover and selection are immediate flat color changes.
- Reader controls, form validation, accessible action semantics, and existing
  keyboard behavior for **forms and buttons** stay functional.
- **Tables remain mouse-selected**: no table-level keyboard `FocusScope`,
  navigation handler, or large focus outline.
- `Radan tiedot` remains visible while a station is selected; the route
  heading and station list do not jump between selections.
- Station time stays in separate hours/minutes/seconds fields, persisted
  in the existing seconds format.
- Native Slint Fluent widgets may have toolkit-internal behavior; this beta
  introduces no custom animation code.

## Source and verification

The uploaded archive used for the audit was an older v0.5.6 working-tree
snapshot; implementation is based on the current GitHub v0.5.7 branch, so the
recent route and station fixes are **not reverted**.

CI builds Linux and Windows, exercises GUI compilation and Rust tests.
To preview the beta locally:

```sh
git fetch origin beta/fluent2-dark-spruce
git switch --track origin/beta/fluent2-dark-spruce
cargo run --features gui --bin as2026-gui
```

This is not a signed/public v0.5.8 release. Review the GUI on real desktops,
especially narrow Wayland tiles, before discussing a merge to `main`.
