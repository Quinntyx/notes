use gtk4::prelude::*;
use gtk4::{gio, glib, Application, ApplicationWindow, Box, Orientation, ListBox, ListBoxRow, Label, ScrolledWindow};
use vte4::{Terminal, PtyFlags, TerminalExtManual};

use crate::note::{NOTES_DIR};

pub fn run_gui() {
    let app = Application::builder()
        .application_id("com.example.notes")
        .build();

    app.connect_activate(|app| {
        // main container
        let main_box = Box::new(Orientation::Horizontal, 0);

        // notes list
        let list = ListBox::new();
        list.set_vexpand(true);

        if let Ok(entries) = std::fs::read_dir(NOTES_DIR) {
            for entry in entries.flatten() {
                if let Some(name_os) = entry.file_name().to_str() {
                    let row = ListBoxRow::new();
                    let label = Label::new(Some(name_os));
                    row.set_child(Some(&label));
                    list.append(&row);
                }
            }
        }

        let scroll = ScrolledWindow::builder()
            .child(&list)
            .vexpand(true)
            .min_content_width(200)
            .build();
        main_box.append(&scroll);

        // terminal for editing
        let terminal = Terminal::new();
        terminal.set_hexpand(true);
        terminal.set_vexpand(true);
        main_box.append(&terminal);

        // row activation opens note in nvim
        let term_clone = terminal.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
                let note_name = label.text().to_string();
                let path = format!("{}/{}", NOTES_DIR, note_name);
                term_clone.spawn_async(
                    PtyFlags::DEFAULT,
                    None::<&str>,
                    &["nvim", &path],
                    &[],
                    glib::SpawnFlags::SEARCH_PATH,
                    || {},
                    -1,
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            }
        });

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Notes GUI")
            .default_width(800)
            .default_height(600)
            .child(&main_box)
            .build();

        window.show();
    });

    app.run();
}
