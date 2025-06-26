# Crate: notes-gui

This crate implements the GTK-based graphical interface.

- Contains window setup, tab management and drawing code in `src/main.rs`.
- Depends on `notes-core` for all data access.

Only put UI and event-handling logic here. Core algorithms should remain in the `notes-core` crate.

## UI design

- Tabs should include close buttons and be reorderable. The graph tab stays pinned at the far left and uses an icon label.
- Graph view actions are image buttons stacked vertically in the bottom-left overlay rather than a top toolbar.
- Hovered graph nodes are tinted and node labels fade in or out based on zoom level.
- Prefer popovers for modal interactions to keep the interface lightweight.
- Keep node label text size constant as you zoom; offset labels outward relative to zoom so they don't overlap the nodes.
- Node format indicators are rendered in ALL CAPS across the interface.
- All UI fonts use the Google Rubik family. Do **not** commit font binaries; the application downloads the files at runtime if missing.
- Follow Google Material Design colors and spacing. Use soft corner radius on buttons and default window background `#FAFAFA`.
- Accent color `#6200EE` should style primary buttons.
- Avoid committing any binary resources such as fonts or images. Fetch them on startup if needed.
