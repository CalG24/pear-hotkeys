use std::rc::Rc;
use tray_icon::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct TrayHandles {
    pub tray: Rc<TrayIcon>, // Rc, not Arc<Mutex<_>> — TrayIcon is !Send, must stay single-threaded.
    pub show_id: MenuId,
    pub quit_id: MenuId,
}

const TRAY_ICON_SIZE: u32 = 32;

fn load_icon(bytes: &[u8]) -> Icon {
    let img = image::load_from_memory(bytes).expect("bundled tray icon is invalid").into_rgba8();
    let resized = if img.width() != TRAY_ICON_SIZE || img.height() != TRAY_ICON_SIZE {
        image::imageops::resize(&img, TRAY_ICON_SIZE, TRAY_ICON_SIZE, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let (w, h) = resized.dimensions();
    Icon::from_rgba(resized.into_raw(), w, h).expect("failed to build tray icon")
}

pub fn build_tray() -> TrayHandles {
    let icon = load_icon(include_bytes!("../assets/icon_neutral.png"));

    let menu = Menu::new();
    let show_item = MenuItem::new("Show / Hide Window", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&show_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&quit_item).unwrap();

    let (show_id, quit_id) = (show_item.id().clone(), quit_item.id().clone());

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_menu_on_right_click(true)
        .with_tooltip("Pear Desktop — starting…")
        .with_icon(icon)
        .build()
        .expect("failed to build tray icon");

    TrayHandles { tray: Rc::new(tray), show_id, quit_id }
}

pub fn icon_for_state(state: &str) -> Icon {
    match state {
        "LIKE" => load_icon(include_bytes!("../assets/icon_liked.png")),
        "DISLIKE" => load_icon(include_bytes!("../assets/icon_disliked.png")),
        _ => load_icon(include_bytes!("../assets/icon_neutral.png")),
    }
}
