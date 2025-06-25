use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Label, ListBox, ListBoxRow, Notebook, Orientation,
    ScrolledWindow, gio, glib,
};
use vte4::{PtyFlags, Terminal, TerminalExtManual};

use crate::note::NOTES_DIR;

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

        // notebook to hold multiple editor tabs
        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);
        main_box.append(&notebook);

        // track open tabs so we don't spawn editors twice
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;
        let open_tabs: Rc<RefCell<HashMap<String, Terminal>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // row activation opens note in a new tab or focuses existing one
        let notebook_clone = notebook.clone();
        let tabs_clone = open_tabs.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
                let note_name = label.text().to_string();
                // check if tab already exists
                if let Some(term) = tabs_clone.borrow().get(&note_name).cloned() {
                    if let Some(page) = notebook_clone.page_num(&term) {
                        notebook_clone.set_current_page(Some(page));
                        return;
                    }
                }

                let path = format!("{}/{}", NOTES_DIR, note_name);
                let term = Terminal::new();
                term.set_hexpand(true);
                term.set_vexpand(true);
                term.spawn_async(
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

                let label_widget = Label::new(Some(&note_name));
                notebook_clone.append_page(&term, Some(&label_widget));
                if let Some(page) = notebook_clone.page_num(&term) {
                    notebook_clone.set_current_page(Some(page));
                }
                tabs_clone.borrow_mut().insert(note_name, term);
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

    // Pass an empty argument list to avoid `g_application_open` from
    // interpreting leftover command line arguments as files to open.
    // This prevents warnings about missing file handlers when running
    // `cargo run gui` from the CLI.
    app.run_with_args::<&str>(&[]);
}
