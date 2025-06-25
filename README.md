# Notes

Notes is a simple note taking application written in Rust. It is inspired by Obsidian but is designed to avoid vendor lock‑in by storing notes as plain Markdown files. The project provides both a command line interface and a GTK4 based graphical UI with an embedded terminal editor.

## Features

- Notes can be written in multiple formats and grouped by filename
- Binary assets live alongside the text notes
- CLI commands for creating and viewing notes
- Interactive graph view shows links between notes and lists available formats
- Text formats open in tabs while binary formats launch with the system default application
- Early project direction aims for integration with external project management tools

## Building

Install the Rust toolchain and GTK development libraries. Then run:

```bash
cargo build
```

To run in debug mode with the graphical interface:

```bash
cargo run
```

## Command Line Usage

```
notes new <title>     Create a new note with the given title
notes show <title>    Display the contents of a note
notes gui             Launch the graphical interface
```

## Contributing

The project uses standard Rust formatting. Please run `cargo fmt` and ensure `cargo build` succeeds before submitting changes. Pull requests with focused commit messages are appreciated.

