# Contributor Guide

This repository contains a small Rust codebase for a note taking application. The project prefers simple, consistent code. Follow these guidelines when adding code or documentation.

## Coding style

- Use `rustfmt` for all Rust files. Run `cargo fmt` before committing.
- Indent with four spaces and keep lines under 100 characters.
- Use `snake_case` for functions and variables and `CamelCase` for type names.
- Organize modules under `src/` and keep each type or utility in its own file when practical.

## Development workflow

- Ensure `cargo build` and `cargo test` (when tests exist) complete without errors before opening a pull request.
- Commit messages should be short, present tense commands such as "Add graph view" or "Fix editor layout".

These conventions help keep contributions uniform and easy to review.

## Crate layout

- **notes-core**: Library and CLI for loading notes, building the graph and managing files. Non-UI logic belongs here.
- **notes-gui**: GTK application providing the graphical interface. It depends on notes-core for data handling.

