use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, DrawingArea, Label, ListBox, ListBoxRow, Notebook,
    Orientation, ScrolledWindow, gio, glib,
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

        // container for list and controls
        let side_box = Box::new(Orientation::Vertical, 5);

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
        side_box.append(&scroll);

        // button to open graph view
        let graph_button = Button::with_label("Graph");
        let app_clone = app.clone();
        graph_button.connect_clicked(move |_| {
            open_graph_window(&app_clone);
        });
        side_box.append(&graph_button);

        main_box.append(&side_box);

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

fn open_graph_window(app: &Application) {
    use crate::graph::build_graph;
    use std::f64::consts::PI;

    let graph = build_graph();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notes Graph")
        .default_width(800)
        .default_height(600)
        .build();

    let area = DrawingArea::new();
    area.set_draw_func(move |_, ctx, width, height| {
        // clear the drawing area first
        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.paint().unwrap();

        let n = graph.nodes.len();
        if n == 0 {
            return;
        }

        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let radius = (width.min(height) as f64) * 0.4;
        let mut positions = Vec::new();

        for i in 0..n {
            let angle = i as f64 / n as f64 * 2.0 * PI;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            positions.push((x, y));
        }

        ctx.set_source_rgb(0.6, 0.6, 0.6);
        ctx.set_line_width(1.0);
        for &(from, to) in &graph.edges {
            let (sx, sy) = positions[from];
            let (tx, ty) = positions[to];
            ctx.move_to(sx, sy);
            ctx.line_to(tx, ty);
            let _ = ctx.stroke();

            // draw arrow head
            let angle = (ty - sy).atan2(tx - sx);
            let arrow_len = 10.0;
            let arrow_ang = std::f64::consts::PI / 8.0; // 22.5 deg
            let lx = tx - arrow_len * (angle - arrow_ang).cos();
            let ly = ty - arrow_len * (angle - arrow_ang).sin();
            let rx = tx - arrow_len * (angle + arrow_ang).cos();
            let ry = ty - arrow_len * (angle + arrow_ang).sin();

            ctx.move_to(lx, ly);
            ctx.line_to(tx, ty);
            ctx.line_to(rx, ry);
            ctx.close_path();
            let _ = ctx.fill();
            ctx.new_path();
        }

        for (i, node) in graph.nodes.iter().enumerate() {
            let (x, y) = positions[i];
            ctx.arc(x, y, 10.0, 0.0, 2.0 * PI);
            ctx.set_source_rgb(0.2, 0.6, 0.86);
            let _ = ctx.fill_preserve();
            ctx.set_source_rgb(0.0, 0.0, 0.0);
            let _ = ctx.stroke();

            ctx.move_to(x + 12.0, y + 4.0);
            let _ = ctx.show_text(&node.name);
            ctx.new_path();
        }
    });

    window.set_child(Some(&area));
    window.show();
}
