use std::env;
use std::process::{self, Command};

use notes_core::note::Note;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "new" => {
            if args.len() < 3 {
                println!("Error: Missing title for 'new' command.");
                print_usage();
                process::exit(1);
            }
            let title = args[2..].join(" "); // Allow titles with spaces
            handle_new_note(&title);
        }
        "show" => {
            if args.len() < 3 {
                println!("Error: Missing title for 'show' command.");
                print_usage();
                process::exit(1);
            }
            let title = args[2..].join(" "); // Allow titles with spaces
            handle_show_note(&title);
        }
        "gui" => {
            let exe =
                env::var("CARGO_BIN_EXE_notes-gui").unwrap_or_else(|_| "notes-gui".to_string());
            if let Err(e) = Command::new(exe).status() {
                eprintln!("Failed to launch GUI: {}", e);
                process::exit(1);
            }
        }
        _ => {
            println!("Error: Unknown command '{}'", command);
            print_usage();
            process::exit(1);
        }
    }
}

fn handle_new_note(title: &str) {
    // For a new note, content is initially empty.
    // Aliases are also empty for now.
    let note = Note::new(title.to_string(), String::new(), None);
    match note.save() {
        Ok(_) => println!("Note '{}' created successfully at {:?}.", title, note.path),
        Err(e) => eprintln!("Error creating note '{}': {}", title, e),
    }
}

fn handle_show_note(title: &str) {
    let note_path = Note::path_from_title(title);
    match Note::load(&note_path) {
        Ok(note) => {
            println!("--- {} ---", note.title);
            println!("{}", note.content);
            if !note.aliases.is_empty() {
                println!("\nAliases: {:?}", note.aliases);
            }
            println!("\n(Source: {:?})", note.path);
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("Error: Note '{}' not found.", title);
            } else {
                eprintln!("Error loading note '{}': {}", title, e);
            }
        }
    }
}

fn print_usage() {
    println!("Usage: notes <command> [arguments]");
    println!("Commands:");
    println!("  new <title>      Create a new note with the given title.");
    println!("  show <title>     Show the content of the note with the given title.");
    println!("  gui              Launch graphical editor.");
    // Future commands:
    // println!("  edit <title>     Open the note with the given title for editing.");
    // println!("  list             List all available notes.");
    // println!("  link <from_title> <to_title> Create a link.");
    // println!("  aliases <title> <alias1> [alias2...] Add aliases to a note.");
}
