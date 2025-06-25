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

        // notebook to hold multiple editor tabs
        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);

        // track open tabs so we don't spawn editors twice
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;
        let open_tabs: Rc<RefCell<HashMap<String, Terminal>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // button to open graph view
        let graph_button = Button::with_label("Graph");
        let notebook_for_graph = notebook.clone();
        let tabs_for_graph = open_tabs.clone();
        graph_button.connect_clicked(move |_| {
            open_graph_tab(&notebook_for_graph, &tabs_for_graph);
        });
        side_box.append(&graph_button);

        // add side box first so it appears on the left
        main_box.append(&side_box);
        main_box.append(&notebook);

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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn open_graph_tab(notebook: &Notebook, open_tabs: &Rc<RefCell<HashMap<String, Terminal>>>) {
    use crate::graph::build_graph;
    use std::f64::consts::PI;

    struct GraphState {
        graph: crate::graph::Graph,
        positions: Vec<(f64, f64)>,
        velocities: Vec<(f64, f64)>,
        pan_x: f64,
        pan_y: f64,
        scale: f64,
    }

    let graph = build_graph();
    let n = graph.nodes.len();

    let mut positions = Vec::new();
    for i in 0..n {
        let angle = i as f64 / n.max(1) as f64 * 2.0 * PI;
        let r = 100.0;
        positions.push((r * angle.cos(), r * angle.sin()));
    }

    let state = Rc::new(RefCell::new(GraphState {
        graph,
        positions,
        velocities: vec![(0.0, 0.0); n],
        pan_x: 0.0,
        pan_y: 0.0,
        scale: 1.0,
    }));

    let area = DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    let draw_state = state.clone();
    area.set_draw_func(move |_, ctx, width, height| {
        let st = draw_state.borrow();
        let graph = &st.graph;
        let positions = &st.positions;

        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.paint().unwrap();

        if graph.nodes.is_empty() {
            return;
        }

        let scale = st.scale;
        let pan_x = st.pan_x + width as f64 / 2.0;
        let pan_y = st.pan_y + height as f64 / 2.0;

        ctx.set_line_width(1.0);
        ctx.set_source_rgb(0.6, 0.6, 0.6);
        for &(from, to) in &graph.edges {
            let (sx, sy) = positions[from];
            let (tx, ty) = positions[to];
            let sx = sx * scale + pan_x;
            let sy = sy * scale + pan_y;
            let tx = tx * scale + pan_x;
            let ty = ty * scale + pan_y;
            ctx.move_to(sx, sy);
            ctx.line_to(tx, ty);
            let _ = ctx.stroke();
        }

        let show_names = scale > 0.5 && graph.nodes.len() < 50;

        for (i, node) in graph.nodes.iter().enumerate() {
            let (x, y) = positions[i];
            let sx = x * scale + pan_x;
            let sy = y * scale + pan_y;
            let radius = 8.0 + (node.links as f64).sqrt() * 2.0;
            ctx.arc(sx, sy, radius * scale.max(0.2), 0.0, 2.0 * PI);
            ctx.set_source_rgb(0.2, 0.6, 0.86);
            let _ = ctx.fill_preserve();
            ctx.set_source_rgb(0.0, 0.0, 0.0);
            let _ = ctx.stroke();

            if show_names {
                ctx.move_to(sx + 12.0 * scale, sy + 4.0 * scale);
                let _ = ctx.show_text(&node.name);
            }
            ctx.new_path();
        }
    });

    // Panning
    let pan_state = state.clone();
    let pan_area = area.clone();
    let gesture = gtk4::GestureDrag::new();
    gesture.connect_drag_update(move |_, dx, dy| {
        let mut st = pan_state.borrow_mut();
        st.pan_x += dx as f64;
        st.pan_y += dy as f64;
        pan_area.queue_draw();
    });
    area.add_controller(gesture);

    // Zooming
    let zoom_state = state.clone();
    let zoom_area = area.clone();
    let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(move |_, _dx, dy| {
        let mut st = zoom_state.borrow_mut();
        let factor = (1.0 - dy as f64 * 0.05).max(0.1);
        st.scale *= factor;
        zoom_area.queue_draw();
        glib::Propagation::Stop
    });
    area.add_controller(scroll);

    // Open note on click
    let click_state = state.clone();
    let click_area = area.clone();
    let notebook_clone = notebook.clone();
    let tabs_clone = open_tabs.clone();
    let click = gtk4::GestureClick::new();
    click.connect_released(move |_, _n, x, y| {
        let st = click_state.borrow();
        let pan_x = st.pan_x + click_area.width() as f64 / 2.0;
        let pan_y = st.pan_y + click_area.height() as f64 / 2.0;
        let gx = (x as f64 - pan_x) / st.scale;
        let gy = (y as f64 - pan_y) / st.scale;

        for (i, node) in st.graph.nodes.iter().enumerate() {
            let (nx, ny) = st.positions[i];
            let radius = 8.0 + (node.links as f64).sqrt() * 2.0;
            let dist2 = (gx - nx).powi(2) + (gy - ny).powi(2);
            if dist2 <= radius.powi(2) {
                // open note
                let note_name = node.name.clone();
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
                return;
            }
        }
    });
    area.add_controller(click);

    // simple physics update
    let sim_area = area.clone();
    let sim_state = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        {
            let mut st = sim_state.borrow_mut();
            let n = st.graph.nodes.len();
            let mut forces = vec![(0.0, 0.0); n];
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = st.positions[i].0 - st.positions[j].0;
                    let dy = st.positions[i].1 - st.positions[j].1;
                    let dist2 = dx * dx + dy * dy + 0.01;
                    let dist = dist2.sqrt();
                    let rep = 2000.0 / dist2;
                    let fx = dx / dist * rep;
                    let fy = dy / dist * rep;
                    forces[i].0 += fx;
                    forces[i].1 += fy;
                    forces[j].0 -= fx;
                    forces[j].1 -= fy;
                }
            }
            for &(a, b) in &st.graph.edges {
                let dx = st.positions[a].0 - st.positions[b].0;
                let dy = st.positions[a].1 - st.positions[b].1;
                let dist = (dx * dx + dy * dy).sqrt();
                let spring = 0.01 * (dist - 100.0);
                let fx = dx / dist * spring;
                let fy = dy / dist * spring;
                forces[a].0 -= fx;
                forces[a].1 -= fy;
                forces[b].0 += fx;
                forces[b].1 += fy;
            }
            for i in 0..n {
                st.velocities[i].0 = (st.velocities[i].0 + forces[i].0) * 0.85;
                st.velocities[i].1 = (st.velocities[i].1 + forces[i].1) * 0.85;
                st.positions[i].0 += st.velocities[i].0 * 0.1;
                st.positions[i].1 += st.velocities[i].1 * 0.1;
            }
        }
        sim_area.queue_draw();
        glib::ControlFlow::Continue
    });

    let label_widget = Label::new(Some("Graph"));
    notebook.append_page(&area, Some(&label_widget));
    if let Some(page) = notebook.page_num(&area) {
        notebook.set_current_page(Some(page));
    }
}
