use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, DrawingArea, Entry, Image, Label, Notebook,
    Orientation, Overlay, Popover, PositionType, gio, glib,
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
        let graph_tab: Rc<RefCell<Option<Overlay>>> = Rc::new(RefCell::new(None));

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
    graph_tab: &Rc<RefCell<Option<Overlay>>>,
    window: &ApplicationWindow,
) {
    use crate::graph::{load_graph_data, update_open_notes};
    use std::f64::consts::PI;

    if let Some(ref existing) = *graph_tab.borrow() {
        if let Some(page) = notebook.page_num(existing) {
            notebook.set_current_page(Some(page));
        }
        return;
    }

    struct GraphState {
        data: crate::graph::GraphData,
        positions: Vec<(f64, f64)>,
        velocities: Vec<(f64, f64)>,
        pan_x: f64,
        pan_y: f64,
        scale: f64,
        hover: Option<usize>,
    }

    fn reset_state(state: &mut GraphState) {
        state.data = load_graph_data();
        let n = state.data.graph.nodes.len();
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
        state.hover = None;
    }

    fn add_node_to_state(state: &mut GraphState, _title: &str) {
        let new_data = load_graph_data();
        let mut new_positions = Vec::new();
        let mut new_velocities = Vec::new();
        for node in &new_data.graph.nodes {
            if let Some(idx) = state
                .data
                .graph
                .nodes
                .iter()
                .position(|n| n.name == node.name)
            {
                new_positions.push(state.positions[idx]);
                new_velocities.push(state.velocities[idx]);
            } else {
                new_positions.push((0.0, 0.0));
                new_velocities.push((0.0, 0.0));
            }
        }
        state.data = new_data;
        state.positions = new_positions;
        state.velocities = new_velocities;
        state.hover = None;
    }

    let mut init = GraphState {
        data: load_graph_data(),
        positions: Vec::new(),
        velocities: Vec::new(),
        pan_x: 0.0,
        pan_y: 0.0,
        scale: 1.0,
        hover: None,
    };
    let n = init.data.graph.nodes.len();
    for i in 0..n {
        let angle = i as f64 / n.max(1) as f64 * 2.0 * PI;
        let r = 100.0;
        init.positions.push((r * angle.cos(), r * angle.sin()));
    }
    init.velocities = vec![(0.0, 0.0); n];
    let state = Rc::new(RefCell::new(init));

    let area = DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    let container = Overlay::new();
    container.set_hexpand(true);
    container.set_vexpand(true);
    container.set_child(Some(&area));

    let button_box = Box::new(Orientation::Vertical, 5);
    button_box.set_halign(gtk4::Align::Start);
    button_box.set_valign(gtk4::Align::End);

    let home_button = Button::new();
    home_button.set_child(Some(&Image::from_icon_name("go-home-symbolic")));
    home_button.set_size_request(40, 40);
    let new_button = Button::new();
    new_button.set_child(Some(&Image::from_icon_name("document-new-symbolic")));
    new_button.set_size_request(40, 40);
    button_box.append(&home_button);
    button_box.append(&new_button);
    container.add_overlay(&button_box);

    let home_state = state.clone();
    let home_area = area.clone();
    home_button.connect_clicked(move |_| {
        let mut st = home_state.borrow_mut();
        reset_state(&mut st);
        home_area.queue_draw();
    });

    let new_state = state.clone();
    let new_area = area.clone();
    new_button.connect_clicked(move |btn| {
        let pop = Popover::new();
        pop.set_has_arrow(true);
        pop.set_position(PositionType::Top);
        pop.set_autohide(true);
        let entry = Entry::new();
        let create_btn = Button::with_label("Create");
        let v = Box::new(Orientation::Vertical, 5);
        v.append(&entry);
        v.append(&create_btn);
        pop.set_child(Some(&v));
        pop.set_parent(btn);
        pop.popup();

        let st_rc = new_state.clone();
        let area_clone = new_area.clone();
        let pop_clone = pop.clone();
        create_btn.connect_clicked(move |_| {
            let title = entry.text().to_string();
            if !title.is_empty() {
                let note = crate::note::Note::new(title.clone(), String::new(), None);
                let _ = note.save();
                let mut st = st_rc.borrow_mut();
                add_node_to_state(&mut st, &title);
                area_clone.queue_draw();
            }
            pop_clone.popdown();
        });
    });

    let draw_state = state.clone();
    area.set_draw_func(move |_, ctx, width, height| {
        let st = draw_state.borrow();
        let graph = &st.data.graph;
        let positions = &st.positions;

        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.paint().unwrap();
        ctx.set_font_size(13.0);

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

        let text_alpha = ((scale - 0.4) * 5.0).clamp(0.0, 1.0);
        let show_names = text_alpha > 0.0 && graph.nodes.len() < 50;

        for (i, node) in graph.nodes.iter().enumerate() {
            let (x, y) = positions[i];
            let sx = x * scale + pan_x;
            let sy = y * scale + pan_y;
            let radius = 8.0 + (node.links as f64).sqrt() * 2.0;
            ctx.arc(sx, sy, radius * scale.max(0.2), 0.0, 2.0 * PI);
            if st.hover == Some(i) {
                ctx.set_source_rgb(0.3, 0.7, 1.0);
            } else {
                ctx.set_source_rgb(0.2, 0.6, 0.86);
            }
            let _ = ctx.fill_preserve();
            ctx.set_source_rgb(0.0, 0.0, 0.0);
            let _ = ctx.stroke();

            let label_alpha = if st.hover == Some(i) { 1.0 } else { text_alpha };
            if st.hover == Some(i) || show_names {
                let offset_x = radius * scale + 8.0;
                let offset_y = 4.0 * scale;
                ctx.move_to(sx + offset_x, sy + offset_y);
                ctx.set_source_rgba(0.0, 0.0, 0.0, label_alpha);
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

    // Hover highlight
    let hover_state = state.clone();
    let hover_area = area.clone();
    let motion = gtk4::EventControllerMotion::new();
    motion.connect_motion(move |_, x, y| {
        let mut st = hover_state.borrow_mut();
        let pan_x = st.pan_x + hover_area.width() as f64 / 2.0;
        let pan_y = st.pan_y + hover_area.height() as f64 / 2.0;
        let gx = (x as f64 - pan_x) / st.scale;
        let gy = (y as f64 - pan_y) / st.scale;
        st.hover = None;
        for (i, node) in st.data.graph.nodes.iter().enumerate() {
            let (nx, ny) = st.positions[i];
            let radius = 8.0 + (node.links as f64).sqrt() * 2.0;
            if (gx - nx).powi(2) + (gy - ny).powi(2) <= radius.powi(2) {
                st.hover = Some(i);
                break;
            }
        }
        hover_area.queue_draw();
    });
    let leave_state = state.clone();
    let leave_area = area.clone();
    motion.connect_leave(move |_| {
        leave_state.borrow_mut().hover = None;
        leave_area.queue_draw();
    });
    area.add_controller(motion);

    // Open note on click
    let click_state = state.clone();
    let click_area = area.clone();
    let notebook_clone = notebook.clone();
    let tabs_clone = open_tabs.clone();
    let click = gtk4::GestureClick::new();
    click.connect_released(move |_, _n, x, y| {
        let note_name_opt = {
            let st = click_state.borrow();
            let pan_x = st.pan_x + click_area.width() as f64 / 2.0;
            let pan_y = st.pan_y + click_area.height() as f64 / 2.0;
            let gx = (x as f64 - pan_x) / st.scale;
            let gy = (y as f64 - pan_y) / st.scale;
            let mut res = None;
            for (i, node) in st.data.graph.nodes.iter().enumerate() {
                let (nx, ny) = st.positions[i];
                let radius = 8.0 + (node.links as f64).sqrt() * 2.0;
                let dist2 = (gx - nx).powi(2) + (gy - ny).powi(2);
                if dist2 <= radius.powi(2) {
                    res = Some(node.name.clone());
                    break;
                }
            }
            res
        };

        let Some(note_name) = note_name_opt else {
            return;
        };
        let path = format!("{}/{}", NOTES_DIR, note_name);

        if let Some(term) = tabs_clone.borrow().get(&note_name).cloned() {
            if let Some(page) = notebook_clone.page_num(&term) {
                notebook_clone.set_current_page(Some(page));
                return;
            }
        }

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

        let label = Label::new(Some(&note_name));
        let close_btn = Button::new();
        close_btn.set_child(Some(&Image::from_icon_name("window-close-symbolic")));
        close_btn.set_size_request(16, 16);
        let tab_box = Box::new(Orientation::Horizontal, 4);
        tab_box.append(&label);
        tab_box.append(&close_btn);
        notebook_clone.append_page(&term, Some(&tab_box));
        notebook_clone.set_tab_reorderable(&term, true);
        if let Some(page) = notebook_clone.page_num(&term) {
            notebook_clone.set_current_page(Some(page));
        }

        let note_key = note_name.clone();
        let nb_clone = notebook_clone.clone();
        let tabs_rc = tabs_clone.clone();
        let term_for_close = term.clone();
        close_btn.connect_clicked(move |_| {
            if let Some(idx) = nb_clone.page_num(&term_for_close) {
                nb_clone.remove_page(Some(idx));
            }
            tabs_rc.borrow_mut().remove(&note_key);
        });
        tabs_clone.borrow_mut().insert(note_name, term);
    });
    area.add_controller(click);

    // simple physics update
    let sim_area = area.clone();
    let sim_state = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        {
            let mut st = sim_state.borrow_mut();
            let n = st.data.graph.nodes.len();
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
            for &(a, b) in &st.data.graph.edges {
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
                if st.velocities[i].0.abs() < 0.001 && forces[i].0.abs() < 0.001 {
                    st.velocities[i].0 *= 0.5;
                }
                if st.velocities[i].1.abs() < 0.001 && forces[i].1.abs() < 0.001 {
                    st.velocities[i].1 *= 0.5;
                }
                st.positions[i].0 += st.velocities[i].0 * 0.1;
                st.positions[i].1 += st.velocities[i].1 * 0.1;
            }
        }
        sim_area.queue_draw();
        glib::ControlFlow::Continue
    });

    let switch_state = state.clone();
    let switch_tabs = open_tabs.clone();
    let switch_area = area.clone();
    let switch_container = container.clone();
    notebook.connect_switch_page(move |nb, _page, idx| {
        if let Some(page_num) = nb.page_num(&switch_container) {
            if page_num == idx {
                let titles: Vec<String> = switch_tabs.borrow().keys().cloned().collect();
                let mut st = switch_state.borrow_mut();
                update_open_notes(&mut st.data, &titles);
                switch_area.queue_draw();
            }
        }
    });

    let graph_icon = Image::from_icon_name("media-playlist-consecutive-symbolic");
    notebook.append_page(&container, Some(&graph_icon));
    notebook.set_tab_reorderable(&container, false);
    if let Some(page) = notebook.page_num(&container) {
        notebook.set_current_page(Some(page));
    }
    *graph_tab.borrow_mut() = Some(container);
}
