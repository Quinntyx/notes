# Notes

Notes is a simple note taking application written in Rust. It is inspired by Obsidian but is designed to avoid vendor lock‑in by storing notes as plain Markdown files. The project provides both a command line interface and a GTK4 based graphical UI with an embedded terminal editor.

## Features

- Notes saved as individual `.md` files under the `notes/` directory
- CLI commands for creating and viewing notes
- Native GTK interface that lists notes and opens them in a tabbed NeoVim terminal
- Interactive graph view showing links between notes
- Early project direction aims for integration with external project management tools

## Building

Install the Rust toolchain and GTK development libraries. Then run:

```bash
cargo build
```

To run in debug mode with the graphical interface:

```bash
cargo run -- gui
```

## Command Line Usage

```
notes new <title>     Create a new note with the given title
notes show <title>    Display the contents of a note
notes gui             Launch the graphical interface
```

## Contributing

The project uses standard Rust formatting. Please run `cargo fmt` and ensure `cargo build` succeeds before submitting changes. Pull requests with focused commit messages are appreciated.

