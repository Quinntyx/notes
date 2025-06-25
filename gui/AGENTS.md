# Crate: notes-gui

This crate implements the GTK-based graphical interface.

- Contains window setup, tab management and drawing code in `src/main.rs`.
- Depends on `notes-core` for all data access.

Only put UI and event-handling logic here. Core algorithms should remain in the `notes-core` crate.
