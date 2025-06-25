use cairo;
use directories::ProjectDirs;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, DrawingArea, Entry, Image, Label, Notebook,
    Orientation, Overlay, Popover, PositionType, gio, glib,
};
use open;
use reqwest::blocking as reqwest;
use vte4::{PtyFlags, Terminal, TerminalExtManual};

use notes_core::note::{set_vault_dir, vault_dir};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use twox_hash::XxHash64;

fn expand_tilde(path: &str) -> PathBuf {
    #[cfg(unix)]
    {
        if path == "~" {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home);
            }
        } else if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(rest);
            }
        }
    }
    PathBuf::from(path)
}

fn is_text_file(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok()
}

fn hash_color(ext: &str) -> (f64, f64, f64) {
    let mut hasher = XxHash64::with_seed(0);
    hasher.write(ext.as_bytes());
    let hash = hasher.finish();
    let r = ((hash >> 0) & 0xFF) as f64 / 255.0;
    let g = ((hash >> 8) & 0xFF) as f64 / 255.0;
    let b = ((hash >> 16) & 0xFF) as f64 / 255.0;
    (r, g, b)
}

fn lighten_color(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let factor = 0.3;
    (
        r + (1.0 - r) * factor,
        g + (1.0 - g) * factor,
        b + (1.0 - b) * factor,
    )
}

fn node_color(node: &notes_core::graph::Node) -> (f64, f64, f64) {
    if node.is_directory() {
        (1.0, 1.0, 1.0)
    } else if let Some(ext) = node.primary_file_format() {
        hash_color(&ext)
    } else {
        hash_color("md")
    }
}

fn ensure_rubik_font() {
    const RUBIK_URL: &str =
        "https://fonts.gstatic.com/s/rubik/v30/iJWZBXyIfDnIV5PNhY1KTN7Z-Yh-B4i1UA.ttf";
    if let Some(proj) = ProjectDirs::from("com", "example", "notes") {
        let font_dir = proj.data_dir().join("fonts");
        let _ = std::fs::create_dir_all(&font_dir);
        let font_path = font_dir.join("Rubik-Regular.ttf");
        if !font_path.exists() {
            if let Ok(bytes) = reqwest::get(RUBIK_URL).and_then(|r| r.bytes()) {
                let _ = std::fs::write(&font_path, bytes);
                let _ = std::process::Command::new("fc-cache")
                    .arg("-f")
                    .arg(&font_dir)
                    .status();
            }
        }
    }
}

pub fn run_gui() {
    ensure_rubik_font();
    let app = Application::builder()
        .application_id("com.example.notes")
        .build();

    if let Some(display) = gdk::Display::default() {
        let provider = gtk4::CssProvider::new();
        let css = "* { font-family: 'Rubik'; }\nwindow { background: #FAFAFA; }\nbutton { border-radius: 4px; padding: 6px 12px; }\n";
        provider.load_from_data(css);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    app.connect_activate(|app| {
        show_dashboard(app);
    });

    // Pass an empty argument list to avoid `g_application_open` from
    // interpreting leftover command line arguments as files to open.
    // This prevents warnings about missing file handlers when running
    // `cargo run gui` from the CLI.
    app.run_with_args::<&str>(&[]);
}

fn show_dashboard(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Select Vault")
        .default_width(400)
        .default_height(200)
        .build();

    let entry = Entry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Vault directory"));
    let button = Button::with_label("Open Vault");
    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.append(&entry);
    vbox.append(&button);
    window.set_child(Some(&vbox));

    button.connect_clicked(
        glib::clone!(@weak window, @weak app, @weak entry => move |_| {
            let path_str = entry.text();
            if !path_str.is_empty() {
                let dir = expand_tilde(path_str.as_str());
                set_vault_dir(dir);
                window.close();
                open_main_window(&app);
            }
        }),
    );
    window.show();
}

fn open_main_window(app: &Application) {
    let notebook = Notebook::new();
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);

    let open_tabs: Rc<RefCell<HashMap<String, Terminal>>> = Rc::new(RefCell::new(HashMap::new()));
    let graph_tab: Rc<RefCell<Option<Overlay>>> = Rc::new(RefCell::new(None));
    let graph_cb: Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>> =
        Rc::new(RefCell::new(None));

    let menu = gio::Menu::new();
    let file_menu = gio::Menu::new();
    file_menu.append(Some("New Note"), Some("app.new_note"));
    file_menu.append(Some("Close Tab"), Some("app.close_tab"));
    file_menu.append(Some("Quit"), Some("app.quit"));
    menu.append_submenu(Some("File"), &file_menu);
    let menu_bar = gtk4::PopoverMenuBar::from_model(Some(&menu));

    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&menu_bar);
    vbox.append(&notebook);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notes GUI")
        .default_width(800)
        .default_height(600)
        .child(&vbox)
        .build();

    let app_clone = app.clone();
    app.add_action_entries(vec![
        gio::ActionEntry::builder("new_note")
            .activate(
                glib::clone!(@weak window, @weak graph_cb => move |_, _, _| {
                    show_new_note_popover(&window, &graph_cb);
                }),
            )
            .build(),
        gio::ActionEntry::builder("close_tab")
            .activate(
                glib::clone!(@weak notebook, @weak open_tabs, @weak graph_tab => move |_,_,_| {
                    close_current_tab(&notebook, &open_tabs, &graph_tab);
                }),
            )
            .build(),
        gio::ActionEntry::builder("quit")
            .activate(move |app: &Application, _, _| {
                app.quit();
            })
            .build(),
    ]);

    app.set_accels_for_action("app.new_note", &["<Primary>n"]);
    app.set_accels_for_action("app.close_tab", &["<Primary>w"]);
    if cfg!(target_os = "macos") {
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }

    let key_controller = gtk4::EventControllerKey::builder()
        .propagation_phase(gtk4::PropagationPhase::Capture)
        .build();
    key_controller.connect_key_pressed(glib::clone!(@weak app_clone => @default-return glib::Propagation::Proceed, move |_, key, _code, state| {
            let modifier = if cfg!(target_os = "macos") { gdk::ModifierType::META_MASK } else { gdk::ModifierType::CONTROL_MASK };
            if state.contains(modifier) {
                if key == gdk::Key::n {
                    app_clone.activate_action("new_note", None);
                    return glib::Propagation::Stop;
                } else if key == gdk::Key::w {
                    app_clone.activate_action("close_tab", None);
                    return glib::Propagation::Stop;
                } else if cfg!(target_os = "macos") && key == gdk::Key::q {
                    app_clone.activate_action("quit", None);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        }));
    window.add_controller(key_controller);

    open_graph_tab(&notebook, &open_tabs, &graph_tab, &graph_cb);

    window.show();
}

fn open_any_path(
    notebook: &Notebook,
    open_tabs: &Rc<RefCell<HashMap<String, Terminal>>>,
    node: &notes_core::graph::Node,
    path: &Path,
) {
    let key = path.to_string_lossy().to_string();
    if is_text_file(path) {
        if let Some(term) = open_tabs.borrow().get(&key).cloned() {
            if let Some(page) = notebook.page_num(&term) {
                notebook.set_current_page(Some(page));
                return;
            }
        }

        let term = Terminal::new();
        term.set_hexpand(true);
        term.set_vexpand(true);
        term.spawn_async(
            PtyFlags::DEFAULT,
            None::<&str>,
            &["nvim", &key],
            &[],
            glib::SpawnFlags::SEARCH_PATH,
            || {},
            -1,
            None::<&gio::Cancellable>,
            |_| {},
        );

        let label = Label::new(path.file_name().and_then(|s| s.to_str()));
        let close_btn = Button::new();
        close_btn.set_child(Some(&Image::from_icon_name("window-close-symbolic")));
        close_btn.set_size_request(16, 16);
        let tab_box = Box::new(Orientation::Horizontal, 4);
        tab_box.append(&label);
        tab_box.append(&close_btn);

        let format_bar = Box::new(Orientation::Horizontal, 4);
        let mut exts: Vec<(String, PathBuf)> = node
            .paths
            .iter()
            .filter_map(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|e| (e.to_ascii_uppercase(), p.clone()))
            })
            .collect();
        exts.sort_by(|a, b| a.0.cmp(&b.0));
        exts.dedup_by(|a, b| a.0 == b.0);
        for (ext_u, path_u) in exts {
            let btn = Button::with_label(&ext_u);
            let nb_clone = notebook.clone();
            let tabs_clone = open_tabs.clone();
            let node_clone = node.clone();
            let path_clone = path_u.clone();
            btn.connect_clicked(move |_| {
                open_any_path(&nb_clone, &tabs_clone, &node_clone, &path_clone);
            });
            format_bar.append(&btn);
        }

        let container = Box::new(Orientation::Vertical, 0);
        container.append(&format_bar);
        container.append(&term);

        notebook.append_page(&container, Some(&tab_box));
        notebook.set_tab_reorderable(&container, true);
        if let Some(page) = notebook.page_num(&container) {
            notebook.set_current_page(Some(page));
        }

        let key_clone = key.clone();
        let nb_clone = notebook.clone();
        let tabs_rc = open_tabs.clone();
        close_btn.connect_clicked(move |_| {
            if let Some(idx) = nb_clone.page_num(&container) {
                nb_clone.remove_page(Some(idx));
            }
            tabs_rc.borrow_mut().remove(&key_clone);
        });
        open_tabs.borrow_mut().insert(key, term);
    } else if let Err(err) = open::that(path) {
        eprintln!("Failed to open {:?}: {}", path, err);
    }
}

fn open_graph_tab(
    notebook: &Notebook,
    open_tabs: &Rc<RefCell<HashMap<String, Terminal>>>,
    graph_tab: &Rc<RefCell<Option<Overlay>>>,
    graph_cb: &Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>>,
) {
    use notes_core::graph::{load_graph_data, update_open_notes};
    use std::f64::consts::PI;

    if let Some(ref existing) = *graph_tab.borrow() {
        if let Some(page) = notebook.page_num(existing) {
            notebook.set_current_page(Some(page));
        }
        return;
    }

    struct GraphState {
        data: notes_core::graph::GraphData,
        positions: Vec<(f64, f64)>,
        velocities: Vec<(f64, f64)>,
        colors: Vec<(f64, f64, f64)>,
        pan_x: f64,
        pan_y: f64,
        scale: f64,
        hover: Option<usize>,
    }

    fn reset_state(state: &mut GraphState) {
        state.data = load_graph_data();
        let n = state.data.graph.nodes.len();
        state.positions.clear();
        state.colors.clear();
        for i in 0..n {
            let angle = i as f64 / n.max(1) as f64 * 2.0 * PI;
            let r = 100.0;
            state.positions.push((r * angle.cos(), r * angle.sin()));
            state.colors.push(node_color(&state.data.graph.nodes[i]));
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
        let mut new_colors = Vec::new();
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
            new_colors.push(node_color(node));
        }
        state.data = new_data;
        state.positions = new_positions;
        state.velocities = new_velocities;
        state.colors = new_colors;
        state.hover = None;
    }

    let mut init = GraphState {
        data: load_graph_data(),
        positions: Vec::new(),
        velocities: Vec::new(),
        colors: Vec::new(),
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
        init.colors.push(node_color(&init.data.graph.nodes[i]));
    }
    init.velocities = vec![(0.0, 0.0); n];
    let state = Rc::new(RefCell::new(init));

    let area = DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);

    let cb_state = state.clone();
    let cb_area = area.clone();
    *graph_cb.borrow_mut() = Some(std::boxed::Box::new(move |title: String| {
        let mut st = cb_state.borrow_mut();
        add_node_to_state(&mut st, &title);
        cb_area.queue_draw();
    }));

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

    let cb_clone = graph_cb.clone();
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

        let pop_clone = pop.clone();
        let entry_clone = entry.clone();
        let cb_inner = cb_clone.clone();
        let do_create = Rc::new(move || {
            let title = entry_clone.text().to_string();
            if !title.is_empty() {
                create_new_note(&title);
                if let Some(cb) = &*cb_inner.borrow() {
                    cb(title.clone());
                }
            }
            pop_clone.popdown();
        });
        let cb = do_create.clone();
        create_btn.connect_clicked(move |_| {
            cb();
        });
        entry.connect_activate(move |_| {
            do_create();
        });
    });

    let draw_state = state.clone();
    area.set_draw_func(move |_, ctx, width, height| {
        let st = draw_state.borrow();
        let graph = &st.data.graph;
        let positions = &st.positions;

        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.paint().unwrap();
        ctx.select_font_face("Rubik", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
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
            let (r, g, b) = st.colors.get(i).copied().unwrap_or((0.2, 0.6, 0.86));
            if node.is_directory() {
                let fill = if st.hover == Some(i) {
                    (0.9, 0.9, 0.9)
                } else {
                    (1.0, 1.0, 1.0)
                };
                ctx.arc(sx, sy, radius * scale.max(0.2), 0.0, 2.0 * PI);
                ctx.set_source_rgb(fill.0, fill.1, fill.2);
                let _ = ctx.fill_preserve();
                ctx.set_source_rgb(0.0, 0.0, 0.0);
                let _ = ctx.stroke();
                ctx.arc(sx, sy, radius * scale.max(0.2) * 0.4, 0.0, 2.0 * PI);
                ctx.set_source_rgb(0.0, 0.0, 0.0);
                let _ = ctx.fill();
                ctx.new_path();
            } else {
                ctx.arc(sx, sy, radius * scale.max(0.2), 0.0, 2.0 * PI);
                if st.hover == Some(i) {
                    let (lr, lg, lb) = lighten_color(r, g, b);
                    ctx.set_source_rgb(lr, lg, lb);
                } else {
                    ctx.set_source_rgb(r, g, b);
                }
                let _ = ctx.fill_preserve();
                ctx.set_source_rgb(0.0, 0.0, 0.0);
                let _ = ctx.stroke();
            }

            let label_alpha = if st.hover == Some(i) { 1.0 } else { text_alpha };
            if st.hover == Some(i) || show_names {
                let offset_x = radius * scale + 8.0;
                let offset_y = -2.0 * scale;
                ctx.move_to(sx + offset_x, sy + offset_y);
                ctx.set_source_rgba(0.0, 0.0, 0.0, label_alpha);
                let _ = ctx.show_text(&node.name);
                let formats: Vec<String> = node
                    .paths
                    .iter()
                    .filter_map(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|s| s.to_ascii_uppercase())
                    })
                    .collect::<Vec<_>>();
                let mut formats = formats;
                formats.sort();
                formats.dedup();
                if !formats.is_empty() {
                    let fmt_text = formats.join(", ");
                    ctx.move_to(sx + offset_x, sy + offset_y + 14.0);
                    ctx.set_source_rgba(0.3, 0.3, 0.3, label_alpha);
                    let _ = ctx.show_text(&fmt_text);
                }
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
        let idx_opt = {
            let st = click_state.borrow();
            st.data.graph.nodes.iter().position(|n| n.name == note_name)
        };

        let mut chosen = None;
        if let Some(idx) = idx_opt {
            let mut st = click_state.borrow_mut();
            let node = &mut st.data.graph.nodes[idx];
            if let Some(md) = node
                .paths
                .iter()
                .find(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
                .cloned()
            {
                chosen = Some((node.clone(), md));
            } else {
                let mut text_paths: Vec<PathBuf> = node
                    .paths
                    .iter()
                    .filter(|p| is_text_file(p))
                    .cloned()
                    .collect();
                text_paths.sort_by_key(|p| {
                    p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_owned())
                        .unwrap_or_default()
                });
                if let Some(p) = text_paths.first() {
                    chosen = Some((node.clone(), p.clone()));
                } else {
                    let new_path = vault_dir().join(format!("{}.md", node.name));
                    let _ = std::fs::File::create(&new_path);
                    node.paths.push(new_path.clone());
                    chosen = Some((node.clone(), new_path));
                }
            }
        }
        if let Some((node, path)) = chosen {
            open_any_path(&notebook_clone, &tabs_clone, &node, &path);
            let mut st = click_state.borrow_mut();
            notes_core::graph::update_open_notes(&mut st.data, &[]);
            click_area.queue_draw();
        }
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

fn create_new_note(title: &str) {
    let note = notes_core::note::Note::new(title.to_string(), String::new(), None);
    let _ = note.save();
}

fn show_new_note_popover(
    window: &ApplicationWindow,
    graph_cb: &Rc<RefCell<Option<std::boxed::Box<dyn Fn(String)>>>>,
) {
    let pop = Popover::new();
    pop.set_has_arrow(false);
    pop.set_autohide(true);
    let entry = Entry::new();
    let create_btn = Button::with_label("Create");
    let v = Box::new(Orientation::Vertical, 5);
    v.append(&entry);
    v.append(&create_btn);
    pop.set_child(Some(&v));
    pop.set_parent(window);
    let rect = gdk::Rectangle::new(
        window.allocated_width() / 2,
        window.allocated_height() / 2,
        1,
        1,
    );
    pop.set_pointing_to(Some(&rect));
    pop.popup();

    let pop_clone = pop.clone();
    let entry_clone = entry.clone();
    let cb_clone = graph_cb.clone();
    let do_create = Rc::new(move || {
        let title = entry_clone.text().to_string();
        if !title.is_empty() {
            create_new_note(&title);
            if let Some(cb) = &*cb_clone.borrow() {
                cb(title.clone());
            }
        }
        pop_clone.popdown();
    });
    let cb = do_create.clone();
    create_btn.connect_clicked(move |_| {
        cb();
    });
    entry.connect_activate(move |_| {
        do_create();
    });
}

fn close_current_tab(
    notebook: &Notebook,
    open_tabs: &Rc<RefCell<HashMap<String, Terminal>>>,
    graph_tab: &Rc<RefCell<Option<Overlay>>>,
) {
    if let Some(current) = notebook.current_page() {
        if let Some(ref graph_widget) = *graph_tab.borrow() {
            if notebook.page_num(graph_widget) == Some(current) {
                return;
            }
        }
        if let Some(widget) = notebook.nth_page(Some(current)) {
            if let Ok(term) = widget.clone().downcast::<Terminal>() {
                let mut remove_key = None;
                for (k, v) in open_tabs.borrow().iter() {
                    if v == &term {
                        remove_key = Some(k.clone());
                        break;
                    }
                }
                if let Some(k) = remove_key {
                    open_tabs.borrow_mut().remove(&k);
                }
            }
        }
        notebook.remove_page(Some(current));
    }
}

fn main() {
    if std::env::var("GSETTINGS_BACKEND")
        .ok()
        .filter(|v| !v.is_empty())
        .is_none()
    {
        unsafe {
            std::env::set_var("GSETTINGS_BACKEND", "memory");
        }
    }
    run_gui();
}
