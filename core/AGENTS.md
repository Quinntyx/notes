# Crate: notes-core

This crate holds the reusable library and command line interface for the project.

- `note.rs` manages loading and saving note files and exposes the vault directory helpers.
- `graph.rs` builds the link graph across all notes.
- `src/main.rs` provides the CLI binary named `notes`.

Place data structures, file I/O code, and algorithms here. User interface code belongs in the `notes-gui` crate.
