use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, Dialog, DialogFlags, DrawingArea, Entry, Label,
    Notebook, Orientation, gio, glib,
};
use vte4::{PtyFlags, Terminal, TerminalExtManual};

use crate::note::NOTES_DIR;

pub fn run_gui() {
    let app = Application::builder()
        .application_id("com.example.notes")
        .build();

    app.connect_activate(|app| {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);

        let open_tabs: Rc<RefCell<HashMap<String, Terminal>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let graph_tab: Rc<RefCell<Option<Box>>> = Rc::new(RefCell::new(None));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Notes GUI")
            .default_width(800)
            .default_height(600)
            .child(&notebook)
            .build();

        open_graph_tab(&notebook, &open_tabs, &graph_tab, &window);

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

fn open_graph_tab(
    notebook: &Notebook,
    open_tabs: &Rc<RefCell<HashMap<String, Terminal>>>,
    graph_tab: &Rc<RefCell<Option<Box>>>,
    window: &ApplicationWindow,
) {
    use crate::graph::build_graph;
    use std::f64::consts::PI;

    if let Some(ref existing) = *graph_tab.borrow() {
        if let Some(page) = notebook.page_num(existing) {
            notebook.set_current_page(Some(page));
        }
        return;
    }

    struct GraphState {
        graph: crate::graph::Graph,
        positions: Vec<(f64, f64)>,
        velocities: Vec<(f64, f64)>,
        pan_x: f64,
        pan_y: f64,
        scale: f64,
    }

    fn reset_state(state: &mut GraphState) {
        state.graph = build_graph();
        let n = state.graph.nodes.len();
        state.positions.clear();
        for i in 0..n {
            let angle = i as f64 / n.max(1) as f64 * 2.0 * PI;
            let r = 100.0;
            state.positions.push((r * angle.cos(), r * angle.sin()));
        }
        state.velocities = vec![(0.0, 0.0); n];
        state.pan_x = 0.0;
        state.pan_y = 0.0;
        state.scale = 1.0;
    }

    let mut init = GraphState {
        graph: build_graph(),
        positions: Vec::new(),
        velocities: Vec::new(),
        pan_x: 0.0,
        pan_y: 0.0,
        scale: 1.0,
    };
    reset_state(&mut init);
    let state = Rc::new(RefCell::new(init));

    let area = DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    let container = Box::new(Orientation::Vertical, 5);
    let toolbar = Box::new(Orientation::Horizontal, 5);
    let home_button = Button::with_label("Home");
    let new_button = Button::with_label("New Note");
    toolbar.append(&home_button);
    toolbar.append(&new_button);
    container.append(&toolbar);
    container.append(&area);

    let home_state = state.clone();
    let home_area = area.clone();
    home_button.connect_clicked(move |_| {
        let mut st = home_state.borrow_mut();
        reset_state(&mut st);
        home_area.queue_draw();
    });

    let new_state = state.clone();
    let new_area = area.clone();
    let window_clone = window.clone();
    new_button.connect_clicked(move |_| {
        let dialog = Dialog::with_buttons(
            Some("New Note"),
            Some(&window_clone),
            DialogFlags::MODAL,
            &[
                ("Create", gtk4::ResponseType::Ok),
                ("Cancel", gtk4::ResponseType::Cancel),
            ],
        );
        let entry = Entry::new();
        dialog.content_area().append(&entry);
        dialog.show();
        let st_rc = new_state.clone();
        let area_clone = new_area.clone();
        dialog.connect_response(move |d, resp| {
            if resp == gtk4::ResponseType::Ok {
                let title = entry.text().to_string();
                if !title.is_empty() {
                    let note = crate::note::Note::new(title, String::new(), None);
                    let _ = note.save();
                    let mut st = st_rc.borrow_mut();
                    reset_state(&mut st);
                    area_clone.queue_draw();
                }
            }
            d.close();
        });
    });

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
    gesture.set_button(gtk4::gdk::BUTTON_MIDDLE);
    let start_pan = Rc::new(RefCell::new((0.0f64, 0.0f64)));
    let start_pan_begin = start_pan.clone();
    gesture.connect_drag_begin(move |_, _x, _y| {
        let st = pan_state.borrow();
        *start_pan_begin.borrow_mut() = (st.pan_x, st.pan_y);
    });
    let pan_state_upd = state.clone();
    let start_pan_update = start_pan.clone();
    gesture.connect_drag_update(move |_, dx, dy| {
        let mut st = pan_state_upd.borrow_mut();
        let (sx, sy) = *start_pan_update.borrow();
        st.pan_x = sx + dx as f64;
        st.pan_y = sy + dy as f64;
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
    notebook.append_page(&container, Some(&label_widget));
    if let Some(page) = notebook.page_num(&container) {
        notebook.set_current_page(Some(page));
    }
    *graph_tab.borrow_mut() = Some(container);
}
