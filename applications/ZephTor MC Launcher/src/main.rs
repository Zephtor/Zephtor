mod core;
mod standalone;

use crate::core::{
    MinecraftInstance, PrismCommand, build_launch_args, default_data_roots, find_instances,
    find_prism_command, normalize_data_root,
};
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, Entry,
    FileChooserAction, FileChooserNative, Label, MessageDialog, MessageType, Orientation,
    PolicyType, ResponseType, STYLE_PROVIDER_PRIORITY_APPLICATION, ScrolledWindow, SearchEntry,
    Separator,
};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const APP_ID: &str = "org.zephtor.MCLauncher";
const PRISM_DOWNLOAD_URL: &str = "https://prismlauncher.org/download/";
const CSS: &str = include_str!("style.css");
const LAUNCHER_ICON: &str = include_str!("../assets/launcher-icon.svg");

#[derive(Default)]
struct LauncherState {
    prism: Option<PrismCommand>,
    custom_root: Option<PathBuf>,
    instances: Vec<MinecraftInstance>,
}

fn read_msa_client_id() -> String {
    std::env::var("ZEPHTOR_MSA_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| standalone::config_value("ZEPHTOR_MSA_CLIENT_ID"))
        .unwrap_or_default()
}

fn save_msa_client_id(client_id: &str) -> std::io::Result<()> {
    let path = config_file()
        .ok_or_else(|| std::io::Error::other("XDG config directory is unavailable"))?
        .with_file_name("launcher.env");
    let parent = path.parent().expect("config path must have a parent");
    fs::create_dir_all(parent)?;
    let existing = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let key = "ZEPHTOR_MSA_CLIENT_ID";
    let mut found = false;
    let mut lines = existing
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&format!("{key}=")) {
                found = true;
                format!("{key}={}", client_id.trim())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>();
    if !found {
        lines.push(format!("{key}={}", client_id.trim()));
    }
    fs::write(path, lines.join("\n") + "\n")
}

#[derive(Clone)]
struct LauncherUi {
    state: Rc<RefCell<LauncherState>>,
    window: ApplicationWindow,
    list: GtkBox,
    empty_notice: Label,
    search: SearchEntry,
    connection_status: Label,
    prism_status: Label,
    root_label: Label,
    open_prism: Button,
}

fn config_file() -> Option<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config_home.join("zephtor-mc-launcher/prism-root.txt"))
}

fn read_custom_root() -> Option<PathBuf> {
    let path = config_file()?;
    let root = fs::read_to_string(path).ok()?;
    let root = root.trim();
    (!root.is_empty()).then(|| normalize_data_root(Path::new(root)))
}

fn save_custom_root(root: &Path) -> std::io::Result<()> {
    let path = config_file().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG config directory is unavailable",
        )
    })?;
    let parent = path.parent().expect("config path must have a parent");
    fs::create_dir_all(parent)?;
    fs::write(path, root.to_string_lossy().as_bytes())
}

fn label(text: &str, css_class: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class(css_class);
    label
}

fn button(text: &str, css_class: &str) -> Button {
    let button = Button::with_label(text);
    button.add_css_class(css_class);
    button
}

fn show_error(parent: &ApplicationWindow, title: &str, detail: &str) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Error)
        .buttons(gtk::ButtonsType::Close)
        .text(title)
        .secondary_text(detail)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn update_instance_list(ui: &LauncherUi) {
    let state = ui.state.borrow();
    if let Some(prism) = &state.prism {
        let kind = if prism.flatpak {
            "Flatpak"
        } else {
            "Systeminstallation"
        };
        ui.prism_status
            .set_text(&format!("● Optionale Prism-Kompatibilität · {kind}"));
        ui.connection_status.set_text(
            "Optionale Prism-Kompatibilität verfügbar. Importierte Instanzen werden über Prism gestartet.",
        );
        ui.open_prism.set_sensitive(true);
    } else {
        ui.prism_status
            .set_text("● Eigenständiger Launcher · Prism optional");
        ui.connection_status.set_text(
            "Der eigenständige Vanilla-Spielstart ist verfügbar. PrismLauncher bleibt optional.",
        );
        ui.open_prism.set_sensitive(false);
    }

    let mut roots = Vec::new();
    if let Some(root) = &state.custom_root {
        roots.push(root.clone());
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.extend(default_data_roots(&home));
    }
    roots.sort();
    roots.dedup();
    let instances = find_instances(&roots);
    ui.root_label.set_text(
        &state
            .custom_root
            .as_ref()
            .map(|root| format!("Eigener Ordner:\n{}", root.display()))
            .unwrap_or_else(|| "Optionale Prism-Importordner werden durchsucht.".to_owned()),
    );
    drop(state);

    ui.state.borrow_mut().instances.clone_from(&instances);
    while let Some(child) = ui.list.first_child() {
        ui.list.remove(&child);
    }

    let query = ui.search.text().to_string().to_lowercase();
    let visible_instances = instances
        .iter()
        .filter(|instance| {
            query.is_empty()
                || instance.name.to_lowercase().contains(&query)
                || instance.minecraft_version.to_lowercase().contains(&query)
                || instance.loader.to_lowercase().contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    ui.empty_notice.set_visible(visible_instances.is_empty());
    if visible_instances.is_empty() && !query.is_empty() {
        ui.empty_notice
            .set_text("Keine Instanz passt zu dieser Suche.");
    } else {
        ui.empty_notice.set_text(
            "Keine Prism-Instanzen gefunden. Du kannst Prism optional zum Import bestehender Instanzen verwenden.",
        );
    }

    for instance in visible_instances {
        ui.list
            .append(&instance_card(&ui.state, instance, &ui.window));
    }
}

fn instance_card(
    state: &Rc<RefCell<LauncherState>>,
    instance: MinecraftInstance,
    window: &ApplicationWindow,
) -> GtkBox {
    let card = GtkBox::new(Orientation::Horizontal, 13);
    card.add_css_class("instance-card");
    let icon = Label::new(Some("⛏"));
    icon.add_css_class("instance-icon");
    card.append(&icon);

    let details = GtkBox::new(Orientation::Vertical, 6);
    details.set_hexpand(true);
    card.append(&details);
    details.append(&label(&instance.name, "instance-name"));

    let metadata = GtkBox::new(Orientation::Horizontal, 7);
    details.append(&metadata);
    let version = if instance.minecraft_version.is_empty() {
        "Minecraft".to_owned()
    } else {
        format!("Minecraft {}", instance.minecraft_version)
    };
    metadata.append(&pill(&version));
    if !instance.loader.is_empty() {
        metadata.append(&pill(&instance.loader));
    }
    metadata.append(&pill(&instance.instance_id));

    let actions = GtkBox::new(Orientation::Horizontal, 7);
    card.append(&actions);

    let play = button("Spielen  ▶", "action");
    play.set_sensitive(state.borrow().prism.is_some());
    let launch_state = Rc::clone(state);
    let launch_window = window.clone();
    let launch_instance = instance.clone();
    play.connect_clicked(move |_| {
        launch_instance_game(&launch_state, &launch_instance, None, &launch_window)
    });
    actions.append(&play);

    let server = Entry::new();
    server.set_placeholder_text(Some("Server-Adresse"));
    server.set_width_chars(17);
    server.set_tooltip_text(Some("Optional: Server-Adresse für den direkten Beitritt"));
    actions.append(&server);
    let join_state = Rc::clone(state);
    let join_window = window.clone();
    let join_instance = instance.clone();
    let server_entry = server.clone();
    let join = button("Beitreten", "secondary");
    join.set_sensitive(state.borrow().prism.is_some());
    join.connect_clicked(move |_| {
        let address = server_entry.text().to_string();
        if address.trim().is_empty() {
            show_error(
                &join_window,
                "Server-Adresse fehlt",
                "Gib zuerst eine Server-Adresse ein.",
            );
            return;
        }
        launch_instance_game(&join_state, &join_instance, Some(&address), &join_window);
    });
    actions.append(&join);

    let open_folder = button("Ordner", "secondary");
    let folder = instance
        .data_root
        .join("instances")
        .join(&instance.instance_id);
    let parent_window = window.clone();
    open_folder.connect_clicked(move |_| {
        let uri = gtk::gio::File::for_path(&folder).uri();
        if let Err(error) =
            gtk::gio::AppInfo::launch_default_for_uri(&uri, None::<&gtk::gio::AppLaunchContext>)
        {
            show_error(
                &parent_window,
                "Instanzordner konnte nicht geöffnet werden",
                &error.to_string(),
            );
        }
    });
    actions.append(&open_folder);

    card
}

fn pill(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.add_css_class("pill");
    label
}

fn launch_instance_game(
    state: &Rc<RefCell<LauncherState>>,
    instance: &MinecraftInstance,
    server: Option<&str>,
    window: &ApplicationWindow,
) {
    let Some(prism) = state.borrow().prism.clone() else {
        show_error(
            window,
            "Eigenständiger Spielstart noch nicht verfügbar",
            "Diese importierte Instanz kann derzeit nur über die optionale Prism-Kompatibilität gestartet werden.",
        );
        return;
    };
    let args = build_launch_args(&prism, &instance.instance_id, &instance.data_root, server);
    if let Err(error) = Command::new(&prism.executable).args(args).spawn() {
        show_error(
            window,
            "Minecraft konnte nicht gestartet werden",
            &error.to_string(),
        );
    }
}

fn install_styles() {
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(application: &Application) {
    install_styles();
    let state = Rc::new(RefCell::new(LauncherState {
        prism: find_prism_command(),
        custom_root: read_custom_root(),
        instances: Vec::new(),
    }));

    let window = ApplicationWindow::builder()
        .application(application)
        .title("ZephTor MC Launcher")
        .default_width(1180)
        .default_height(780)
        .build();
    window.set_size_request(820, 600);

    let shell = GtkBox::new(Orientation::Vertical, 0);
    shell.add_css_class("app-shell");
    window.set_child(Some(&shell));

    let topbar = GtkBox::new(Orientation::Horizontal, 13);
    topbar.add_css_class("topbar");
    let logo = Label::new(Some("Z"));
    logo.add_css_class("brand-mark");
    topbar.append(&logo);
    let brand = GtkBox::new(Orientation::Vertical, 3);
    brand.append(&label("ZEPHTOR MC LAUNCHER", "brand-title"));
    brand.append(&label("DEIN MINECRAFT. DEIN SETUP.", "brand-subtitle"));
    topbar.append(&brand);
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    topbar.append(&spacer);
    let prism_status = label("", "muted");
    topbar.append(&prism_status);
    shell.append(&topbar);

    let scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vexpand(true)
        .build();
    shell.append(&scroller);
    let content = GtkBox::new(Orientation::Vertical, 22);
    content.set_margin_top(28);
    content.set_margin_bottom(28);
    content.set_margin_start(30);
    content.set_margin_end(30);
    scroller.set_child(Some(&content));

    let hero = GtkBox::new(Orientation::Horizontal, 20);
    hero.add_css_class("hero");
    let hero_info = GtkBox::new(Orientation::Vertical, 10);
    hero_info.set_hexpand(true);
    hero_info.append(&label("MINECRAFT JAVA EDITION  /  RUST + GTK4", "eyebrow"));
    let heading = label("Deine Welten.\nEin Klick.", "hero-title");
    heading.set_wrap(true);
    hero_info.append(&heading);
    let description = label(
        "Ein eigenständiger Minecraft-Launcher in Rust. PrismLauncher kann optional als Kompatibilitäts-Backend verwendet werden.",
        "hero-description",
    );
    description.set_wrap(true);
    hero_info.append(&description);
    hero.append(&hero_info);

    let hero_actions = GtkBox::new(Orientation::Vertical, 9);
    hero_actions.set_valign(Align::Center);
    let install_vanilla = button("Minecraft installieren", "action");
    let install_status = label("", "muted");
    install_status.set_wrap(true);
    let status_for_install = install_status.clone();
    let button_for_install = install_vanilla.clone();
    install_vanilla
        .connect_clicked(move |_| install_latest_release(&status_for_install, &button_for_install));
    hero_actions.append(&install_vanilla);
    let launch_vanilla = button("Standalone starten", "secondary");
    let status_for_launch = install_status.clone();
    let button_for_launch = launch_vanilla.clone();
    launch_vanilla
        .connect_clicked(move |_| launch_latest_from_ui(&status_for_launch, &button_for_launch));
    hero_actions.append(&launch_vanilla);
    hero_actions.append(&install_status);
    let open_prism = button("PrismLauncher öffnen", "action");
    open_prism.set_sensitive(state.borrow().prism.is_some());
    let open_state = Rc::clone(&state);
    open_prism.connect_clicked(move |_| open_prism_launcher(&open_state));
    hero_actions.append(&open_prism);
    let install_prism = button("Prism optional installieren", "secondary");
    install_prism.connect_clicked(|_| open_url(PRISM_DOWNLOAD_URL));
    hero_actions.append(&install_prism);
    hero.append(&hero_actions);
    let hero_revealer = gtk::Revealer::new();
    hero_revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    hero_revealer.set_transition_duration(360);
    hero_revealer.set_child(Some(&hero));
    hero_revealer.set_reveal_child(false);
    content.append(&hero_revealer);

    let body = GtkBox::new(Orientation::Horizontal, 20);
    body.set_vexpand(true);
    content.append(&body);
    let instances_panel = GtkBox::new(Orientation::Vertical, 14);
    instances_panel.set_hexpand(true);
    instances_panel.set_valign(Align::Start);
    body.append(&instances_panel);

    let heading_row = GtkBox::new(Orientation::Horizontal, 12);
    let instances_heading = label("Deine Instanzen", "section-title");
    heading_row.append(&instances_heading);
    let heading_spacer = GtkBox::new(Orientation::Horizontal, 0);
    heading_spacer.set_hexpand(true);
    heading_row.append(&heading_spacer);
    let search = SearchEntry::new();
    search.set_placeholder_text(Some("Instanzen filtern"));
    search.set_width_chars(20);
    heading_row.append(&search);
    let refresh = button("Aktualisieren", "secondary");
    heading_row.append(&refresh);
    instances_panel.append(&heading_row);

    let list = GtkBox::new(Orientation::Vertical, 10);
    instances_panel.append(&list);
    let empty_notice = label(
        "Keine Instanzen gefunden. Du kannst eine Vanilla-Version herunterladen und direkt starten; Prism-Instanzen bleiben optional importierbar.",
        "notice",
    );
    empty_notice.set_wrap(true);
    instances_panel.append(&empty_notice);

    let side = GtkBox::new(Orientation::Vertical, 12);
    side.add_css_class("side-panel");
    side.set_valign(Align::Start);
    side.append(&label("Launcher-Verbindung", "side-heading"));
    let connection_status = label("", "muted");
    connection_status.set_wrap(true);
    side.append(&connection_status);
    let choose_root = button("Prism-Ordner importieren", "secondary");
    side.append(&choose_root);
    let root_label = label("", "footer-note");
    root_label.set_wrap(true);
    root_label.set_selectable(true);
    side.append(&root_label);
    side.append(&Separator::new(Orientation::Horizontal));
    side.append(&label("Microsoft-Anmeldung", "side-heading"));
    let client_id = Entry::new();
    client_id.set_placeholder_text(Some("Öffentliche Microsoft-App-ID"));
    client_id.set_text(&read_msa_client_id());
    side.append(&client_id);
    let save_client_id = button("App-ID speichern", "secondary");
    let client_id_status = label("", "footer-note");
    let client_id_input = client_id.clone();
    let client_id_status_out = client_id_status.clone();
    save_client_id.connect_clicked(move |_| match save_msa_client_id(&client_id_input.text()) {
        Ok(()) => client_id_status_out.set_text("Microsoft-App-ID gespeichert."),
        Err(error) => client_id_status_out.set_text(&format!("Speichern fehlgeschlagen: {error}")),
    });
    side.append(&save_client_id);
    side.append(&client_id_status);
    side.append(&Separator::new(Orientation::Horizontal));
    let note = label(
        "Native GTK-Oberfläche ohne Webview. Vanilla-Downloads und Microsoft-Geräteanmeldung sind integriert. Zum Spielen brauchst du Java und eine öffentliche Microsoft-App-ID.",
        "footer-note",
    );
    note.set_wrap(true);
    side.append(&note);
    body.append(&side);

    let ui = LauncherUi {
        state,
        window: window.clone(),
        list,
        empty_notice,
        search,
        connection_status,
        prism_status,
        root_label,
        open_prism,
    };
    let refresh_ui = ui.clone();
    refresh.connect_clicked(move |_| {
        refresh_ui.state.borrow_mut().prism = find_prism_command();
        update_instance_list(&refresh_ui);
    });
    let search_ui = ui.clone();
    ui.search
        .connect_search_changed(move |_| update_instance_list(&search_ui));
    let root_ui = ui.clone();
    choose_root.connect_clicked(move |_| choose_prism_root(&root_ui));

    update_instance_list(&ui);
    window.present();
    hero_revealer.set_reveal_child(true);
}

fn choose_prism_root(ui: &LauncherUi) {
    let chooser = FileChooserNative::new(
        Some("Optionalen PrismLauncher-Ordner importieren"),
        Some(&ui.window),
        FileChooserAction::SelectFolder,
        Some("Ordner auswählen"),
        Some("Abbrechen"),
    );
    let ui = ui.clone();
    chooser.connect_response(move |dialog, response| {
        if response == ResponseType::Accept
            && let Some(path) = dialog.file().and_then(|file| file.path())
        {
            let root = normalize_data_root(&path);
            match save_custom_root(&root) {
                Ok(()) => ui.state.borrow_mut().custom_root = Some(root),
                Err(error) => show_error(
                    &ui.window,
                    "Ordner konnte nicht gespeichert werden",
                    &error.to_string(),
                ),
            }
            update_instance_list(&ui);
        }
        dialog.destroy();
    });
    chooser.show();
}

fn open_prism_launcher(state: &Rc<RefCell<LauncherState>>) {
    let Some(prism) = state.borrow().prism.clone() else {
        open_url(PRISM_DOWNLOAD_URL);
        return;
    };
    let mut command = Command::new(&prism.executable);
    if prism.flatpak {
        command.args(["run", core::PRISM_FLATPAK_ID]);
    }
    if let Some(root) = &state.borrow().custom_root {
        command.arg("--dir").arg(root);
    }
    if let Err(error) = command.spawn() {
        eprintln!("Could not open PrismLauncher: {error}");
    }
}

fn open_url(url: &str) {
    let status = Command::new("xdg-open").arg(url).spawn();
    if let Err(error) = status {
        eprintln!("Could not open {url}: {error}");
    }
}

fn install_latest_release(status: &Label, install_button: &Button) {
    status.set_text("Lade Version, Bibliotheken und Assets. Das kann einige Minuten dauern …");
    install_button.set_sensitive(false);
    let status = status.clone();
    let install_button = install_button.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result: Result<String, String> = (|| {
            let (latest, versions) = standalone::fetch_versions()?;
            let release = versions
                .iter()
                .find(|version| version.id == latest)
                .ok_or_else(|| format!("Release {latest} fehlt in der Versionsliste."))?;
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME ist nicht gesetzt")?;
            standalone::install_client(&standalone::game_directory(&home), release)?;
            Ok(format!("Minecraft {latest} wurde installiert."))
        })();
        let _ = sender.send(result);
    });

    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(message)) => {
                status.set_text(&message);
                install_button.set_sensitive(true);
                gtk::glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                status.set_text(&format!("Installation fehlgeschlagen: {error}"));
                install_button.set_sensitive(true);
                gtk::glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                status.set_text("Installation wurde unerwartet beendet.");
                install_button.set_sensitive(true);
                gtk::glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
        }
    });
}

fn launch_latest_from_ui(status: &Label, launch_button: &Button) {
    status.set_text("Prüfe Minecraft-Version und Konto …");
    launch_button.set_sensitive(false);
    let status = status.clone();
    let launch_button = launch_button.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let notify_sender = sender.clone();
        let result = standalone::launch_latest(&|message| {
            let _ = notify_sender.send(message.to_owned());
        });
        if let Err(error) = result {
            let _ = sender.send(format!("Start fehlgeschlagen: {error}"));
        } else {
            let _ = sender.send("Minecraft wurde gestartet.".to_owned());
        }
    });

    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(message) => status.set_text(&message),
            Err(mpsc::TryRecvError::Disconnected) => {
                launch_button.set_sensitive(true);
                return gtk::glib::ControlFlow::Break;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        gtk::glib::ControlFlow::Continue
    });
}

fn install_desktop_entry() -> std::io::Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    let local_bin = home.join(".local/bin");
    let applications = home.join(".local/share/applications");
    let icons = home.join(".local/share/icons/hicolor/scalable/apps");
    fs::create_dir_all(&local_bin)?;
    fs::create_dir_all(&applications)?;
    fs::create_dir_all(&icons)?;

    let installed_binary = local_bin.join("zephtor-mc-launcher");
    fs::copy(std::env::current_exe()?, &installed_binary)?;
    fs::write(
        icons.join("org.zephtor.MCLauncher.svg"),
        LAUNCHER_ICON.as_bytes(),
    )?;
    let escaped_binary = installed_binary
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=ZephTor MC Launcher\n\
         Comment=Eigenständiger Minecraft-Launcher mit optionaler Prism-Kompatibilität\n\
         Exec=\"{escaped_binary}\"\n\
         Icon=org.zephtor.MCLauncher\n\
         Terminal=false\n\
         Categories=Game;\n\
         StartupNotify=true\n"
    );
    fs::write(
        applications.join("org.zephtor.MCLauncher.desktop"),
        desktop_entry,
    )?;
    println!("Desktop-Eintrag installiert. Du findest ZephTor MC Launcher im Anwendungsmenü.");
    Ok(())
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(result) = standalone::run_cli(&args) {
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    if std::env::args().nth(1).as_deref() == Some("--install-desktop") {
        if let Err(error) = install_desktop_entry() {
            eprintln!("Desktop-Eintrag konnte nicht installiert werden: {error}");
            std::process::exit(1);
        }
        return;
    }

    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(build_ui);
    let _ = application.run();
}
