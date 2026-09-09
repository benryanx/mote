# Mote studio roadmap

The ambition is a professional pixel art workflow with Aseprite's approachability.
The current alpha establishes editable artwork, animation and native integration.
The following features are planned, not implemented or implied by existing UI.

## Editing depth

- Pixel-perfect freehand corner cleanup, pressure-aware tablet input, custom brushes.
- Shading ink, color replacement, gradient tolerance/connected-region filling
  and custom dither matrices. Linear/radial gradients, Bayer patterns and palette
  ramps are implemented.
- Lasso, magic wand, additive/subtractive selections and drag-to-move transforms.
- Rotations, advanced pixel-art scaling algorithms and automatic trim.
  Canvas resize/crop and nearest-neighbor sprite scaling are implemented.
- Layer groups, blend modes, merge/flatten, masks and reference layers.
- Recover-on-start UI, persistent open-document sessions and recovery retention.
  Independent document tabs and per-document recovery snapshots are implemented.
- OS clipboard images, palette import/export, indexed color mode and color profiles.

## Animation and game assets

- Cel grid timeline with thumbnails, linked cels, drag reorder and multi-frame edits.
- Tags, loop ranges, ping-pong playback and adjustable previous/next onion skins.
- Tile sets, tile maps, seamless editing, isometric guides and slices.
- Aseprite file interoperability, GIF import and atlas packing with JSON metadata.
- Named export presets, packed atlas metadata and video export. Export scaling,
  common trim, padding, frame ranges, sequences and format selection are implemented.

## Release quality

- Incremental GPU texture updates and sparse tile/delta undo for large artwork.
- Fuzz project/image input and test failed saves, exhausted disks and crash recovery.
- End-to-end tablet, portal, fractional scaling and light/dark theme testing.
- Keyboard focus and screen reader audits; configurable mouse gestures. Command
  and tool keyboard bindings, conflict detection and portable profiles are implemented.
- Continue modularizing input gestures, persistence and exporters; workspace,
  palettes and frame interactions now have dedicated modules.
- Signed/versioned release artifacts, reproducible packaging, clean-install test.
- Validate QML in a real Quattro session and test enable/disable/removal lifecycle.
- Public repository, marketplace ID/name checks, accurate preview and submission review.

No marketplace submission or Aseprite compatibility is claimed by the alpha.
