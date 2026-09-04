// src/bin/tray_test.rs
use tray_icon::{
    menu::MenuEvent,
    menu::{Menu, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};

fn main() {
    let icon = tray_icon::Icon::from_rgba(vec![255, 0, 0, 255], 1, 1).unwrap();
    let menu = Menu::new();
    let item = MenuItem::new("Quit", true, None);
    menu.append(&item).unwrap();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("test")
        .build()
        .unwrap();

    TrayIconEvent::set_event_handler(Some(|e| println!("TRAY: {e:?}")));
    MenuEvent::set_event_handler(Some(|e| println!("MENU: {e:?}")));

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let mut msg = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
