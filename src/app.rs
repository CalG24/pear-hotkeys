use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::{menu::MenuEvent, MouseButton, MouseButtonState, TrayIcon, TrayIconEvent};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SetForegroundWindow, SetTimer, ShowWindow, SW_HIDE, SW_SHOWDEFAULT,
};

use crate::api::{self, HotkeyAction};
use crate::audio;
use crate::autostart;
use crate::config::Config;
use crate::hotkeys::{self, HotKeyState, Hotkeys};
use crate::notification;
use crate::tray::{self, TrayHandles};

#[derive(Default, Clone)]
struct SharedState {
    connected: bool,
    like_state: String,
    last_error: Option<String>,
}

#[derive(Clone)]
struct HotkeySettings {
    api_url: String,
    sound_enabled: bool,
    show_notifications: bool,
    debounce_ms: u64,
    like_id: u32,
    dislike_id: u32,
}

fn human_state(state: &str) -> &str {
    match state {
        "LIKE" => "Liked ❤️",
        "DISLIKE" => "Disliked 👎",
        _ => "Connected",
    }
}

fn toggle_window_raw(hwnd_raw: isize, visible: &Arc<AtomicBool>) {
    let now_visible = !visible.load(Ordering::SeqCst);
    unsafe {
        if now_visible {
            ShowWindow(hwnd_raw as HWND, SW_SHOWDEFAULT);
            SetForegroundWindow(hwnd_raw as HWND);
        } else {
            ShowWindow(hwnd_raw as HWND, SW_HIDE);
        }
    }
    visible.store(now_visible, Ordering::SeqCst);
}

// --- Tray icon refresh, main-thread-only ------------------------------
// TrayIcon is !Send (it wraps Rc<RefCell<_>> internally), so it can never
// be shared into a tokio worker thread. Instead, async tasks only ever
// touch `SharedState` (genuinely Send+Sync). This thread-local + SetTimer
// callback runs strictly on the main thread — the same thread that owns
// TrayIcon — and polls SharedState periodically to keep the icon in sync,
// including while the window is hidden (SetTimer's callback fires as part
// of the same message-pump dispatch already proven to run continuously
// even when minimised, via the tray/menu event tests earlier).
thread_local! {
    static TRAY_ICON: RefCell<Option<Rc<TrayIcon>>> = const { RefCell::new(None) };
    static TRAY_SHARED: RefCell<Option<Arc<Mutex<SharedState>>>> = const { RefCell::new(None) };
    static TRAY_LAST_STATE: RefCell<String> = const { RefCell::new(String::new()) };
}

unsafe extern "system" fn tray_refresh_timer_proc(_hwnd: HWND, _msg: u32, _id: usize, _time: u32) {
    let (state, connected) = TRAY_SHARED.with(|s| {
        if let Some(shared) = s.borrow().as_ref() {
            let g = shared.lock().unwrap();
            (g.like_state.clone(), g.connected)
        } else {
            (String::new(), false)
        }
    });

    let state_changed = TRAY_LAST_STATE.with(|last| {
        let mut last = last.borrow_mut();
        if *last != state {
            *last = state.clone();
            true
        } else {
            false
        }
    });

    TRAY_ICON.with(|t| {
        if let Some(tray) = t.borrow().as_ref() {
            if state_changed {
                let _ = tray.set_icon(Some(tray::icon_for_state(&state)));
            }
            let tooltip = if connected {
                format!("Pear Desktop — {}", human_state(&state))
            } else {
                "Pear Desktop — ⚠ not connected".to_string()
            };
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    });
}

async fn do_trigger(
    client: reqwest::Client,
    base_url: String,
    shared: Arc<Mutex<SharedState>>,
    sound_enabled: bool,
    show_notifications: bool,
    action: HotkeyAction,
) {
    let result = match action {
        HotkeyAction::Like => api::post_like(&client, &base_url).await,
        HotkeyAction::Dislike => api::post_dislike(&client, &base_url).await,
    };
    match result {
        Ok(()) => {
            let new_state = match api::get_like_state(&client, &base_url).await {
                Ok(state) => {
                    let mut s = shared.lock().unwrap();
                    s.connected = true;
                    s.like_state = state.state.clone();
                    Some(state.state)
                }
                Err(_) => None,
            };

            if show_notifications {
                let (title, body) = match (action, new_state.as_deref()) {
                    (HotkeyAction::Like, Some("LIKE")) => ("YouTube Music", "Liked ❤️"),
                    (HotkeyAction::Like, Some(_)) => ("YouTube Music", "Unliked 🤍"),
                    (HotkeyAction::Dislike, Some("DISLIKE")) => ("YouTube Music", "Disliked 👎"),
                    (HotkeyAction::Dislike, Some(_)) => ("YouTube Music", "Removed dislike 👍"),
                    (_, None) => ("YouTube Music", "Updated"),
                };
                std::thread::spawn(move || notification::show(title, body));
            }
            if sound_enabled {
                audio::play_notification_sound();
            }
        }
        Err(e) => {
            let mut s = shared.lock().unwrap();
            s.connected = false;
            s.last_error = Some(e.to_string());
        }
    }
}

pub struct PearApp {
    config: Config,
    edit_config: Config,
    hotkeys: Hotkeys,
    hotkey_settings: Arc<Mutex<HotkeySettings>>,
    // These four are only ever accessed through clones captured by the
    // hotkey/health-check closures — never via `self.field` — but must
    // stay alive here for their side effects (runtime kept running, tray
    // icon kept from being destroyed, etc.), so silence dead-code rather
    // than remove them.
    #[allow(dead_code)]
    last_trigger: Arc<Mutex<Instant>>,
    #[allow(dead_code)]
    tray: TrayHandles,
    #[allow(dead_code)]
    rt: tokio::runtime::Runtime,
    #[allow(dead_code)]
    client: reqwest::Client,
    shared: Arc<Mutex<SharedState>>,
    window_visible: Arc<AtomicBool>,
    capturing_like: bool,
    capturing_dislike: bool,
    status_message: Option<String>,
    hwnd: HWND,
}

impl PearApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let hotkeys = Hotkeys::new(&config)
            .expect("failed to register global hotkeys — check they aren't used elsewhere");

        let hwnd: HWND = match cc.window_handle().expect("no window handle").as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get() as HWND,
            _ => panic!("unsupported platform"),
        };
        let hwnd_raw = hwnd as isize;

        let tray_handles = tray::build_tray();
        let window_visible = Arc::new(AtomicBool::new(false));

        {
            let visible = window_visible.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    toggle_window_raw(hwnd_raw, &visible);
                }
            }));
        }
        {
            let visible = window_visible.clone();
            let show_id = tray_handles.show_id.clone();
            let quit_id = tray_handles.quit_id.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == show_id {
                    toggle_window_raw(hwnd_raw, &visible);
                } else if event.id == quit_id {
                    std::process::exit(0);
                }
            }));
        }

        let shared = Arc::new(Mutex::new(SharedState {
            connected: false,
            like_state: String::new(),
            last_error: None,
        }));

        // Wire the main-thread-only tray refresh mechanism.
        TRAY_ICON.with(|t| *t.borrow_mut() = Some(tray_handles.tray.clone()));
        TRAY_SHARED.with(|s| *s.borrow_mut() = Some(shared.clone()));
        unsafe {
            SetTimer(std::ptr::null_mut(), 0, 500, Some(tray_refresh_timer_proc));
        }

        let hotkey_settings = Arc::new(Mutex::new(HotkeySettings {
            api_url: config.api_url.clone(),
            sound_enabled: config.sound_enabled,
            show_notifications: config.show_notifications,
            debounce_ms: config.debounce_ms,
            like_id: hotkeys.like_id(),
            dislike_id: hotkeys.dislike_id(),
        }));
        let last_trigger = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60)));

        {
            let settings = hotkey_settings.clone();
            let last_trigger = last_trigger.clone();
            let client = client.clone();
            let shared = shared.clone();
            let rt_handle = rt.handle().clone();
            let ctx = cc.egui_ctx.clone();

            hotkeys::GlobalHotKeyEvent::set_event_handler(Some(
                move |event: hotkeys::GlobalHotKeyEvent| {
                    if event.state != HotKeyState::Pressed {
                        return;
                    }

                    let s = settings.lock().unwrap().clone();
                    let action = if event.id == s.like_id {
                        HotkeyAction::Like
                    } else if event.id == s.dislike_id {
                        HotkeyAction::Dislike
                    } else {
                        return;
                    };

                    {
                        let mut lt = last_trigger.lock().unwrap();
                        let now = Instant::now();
                        if now.duration_since(*lt) < Duration::from_millis(s.debounce_ms) {
                            return;
                        }
                        *lt = now;
                    }

                    let client = client.clone();
                    let shared = shared.clone();
                    let ctx = ctx.clone();
                    rt_handle.spawn(async move {
                        do_trigger(
                            client,
                            s.api_url,
                            shared,
                            s.sound_enabled,
                            s.show_notifications,
                            action,
                        )
                        .await;
                        ctx.request_repaint();
                    });
                },
            ));
        }

        {
            let shared = shared.clone();
            let client = client.clone();
            let base_url = config.api_url.clone();
            let ctx = cc.egui_ctx.clone();
            rt.spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    let result = api::get_like_state(&client, &base_url).await;
                    let mut s = shared.lock().unwrap();
                    match result {
                        Ok(state) => {
                            s.connected = true;
                            s.like_state = state.state;
                            s.last_error = None;
                        }
                        Err(e) => {
                            s.connected = false;
                            s.last_error = Some(e.to_string());
                        }
                    }
                    drop(s);
                    ctx.request_repaint();
                }
            });
        }

        if !config.start_minimised {
            unsafe {
                ShowWindow(hwnd, SW_SHOWDEFAULT);
                SetForegroundWindow(hwnd);
            }
            window_visible.store(true, Ordering::SeqCst);
        }

        if let Err(e) = autostart::set_autostart(config.start_with_windows) {
            tracing::warn!("could not sync autostart: {e:#}");
        }

        Self {
            edit_config: config.clone(),
            config,
            hotkeys,
            hotkey_settings,
            last_trigger,
            tray: tray_handles,
            rt,
            client,
            shared,
            window_visible,
            capturing_like: false,
            capturing_dislike: false,
            status_message: None,
            hwnd,
        }
    }

    fn set_window_visible(&mut self, visible: bool) {
        unsafe {
            if visible {
                ShowWindow(self.hwnd, SW_SHOWDEFAULT);
                SetForegroundWindow(self.hwnd);
            } else {
                ShowWindow(self.hwnd, SW_HIDE);
            }
        }
        self.window_visible.store(visible, Ordering::SeqCst);
    }
}

fn capture_combo(ctx: &egui::Context) -> Option<String> {
    ctx.input(|i| {
        i.events.iter().find_map(|event| {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                let mut parts = Vec::new();
                if modifiers.ctrl {
                    parts.push("Ctrl".to_string());
                }
                if modifiers.alt {
                    parts.push("Alt".to_string());
                }
                if modifiers.shift {
                    parts.push("Shift".to_string());
                }
                if parts.is_empty() {
                    return None;
                }
                parts.push(format!("{key:?}"));
                Some(parts.join("+"))
            } else {
                None
            }
        })
    })
}

impl eframe::App for PearApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && self.config.minimise_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.set_window_visible(false);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pear Desktop — Hotkeys");
            ui.add_space(8.0);

            let s = self.shared.lock().unwrap().clone();
            ui.horizontal(|ui| {
                ui.label("Status:");
                if s.connected {
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 170, 0),
                        format!("● {}", human_state(&s.like_state)),
                    );
                } else {
                    ui.colored_label(egui::Color32::RED, "● Not connected");
                }
            });
            if let Some(err) = &s.last_error {
                ui.small(format!("Last error: {err}"));
            }

            ui.separator();
            ui.label("API");
            ui.horizontal(|ui| {
                ui.label("Server URL:");
                ui.text_edit_singleline(&mut self.edit_config.api_url);
            });

            ui.separator();
            ui.label("Hotkeys");
            ui.horizontal(|ui| {
                ui.label("Like:");
                ui.monospace(&self.edit_config.hotkey_like);
                if ui
                    .button(if self.capturing_like {
                        "Press keys…"
                    } else {
                        "Rebind"
                    })
                    .clicked()
                {
                    self.capturing_like = true;
                    self.capturing_dislike = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Dislike:");
                ui.monospace(&self.edit_config.hotkey_dislike);
                if ui
                    .button(if self.capturing_dislike {
                        "Press keys…"
                    } else {
                        "Rebind"
                    })
                    .clicked()
                {
                    self.capturing_dislike = true;
                    self.capturing_like = false;
                }
            });
            if self.capturing_like || self.capturing_dislike {
                if let Some(combo) = capture_combo(ctx) {
                    if self.capturing_like {
                        self.edit_config.hotkey_like = combo;
                        self.capturing_like = false;
                    } else {
                        self.edit_config.hotkey_dislike = combo;
                        self.capturing_dislike = false;
                    }
                }
            }

            ui.separator();
            ui.label("Behaviour");
            ui.horizontal(|ui| {
                ui.label("Debounce (ms):");
                ui.add(egui::DragValue::new(&mut self.edit_config.debounce_ms).range(0..=5000));
            });
            ui.checkbox(&mut self.edit_config.sound_enabled, "Play sound on action");
            ui.checkbox(
                &mut self.edit_config.show_notifications,
                "Show notification on action",
            );
            ui.checkbox(
                &mut self.edit_config.start_with_windows,
                "Start with Windows",
            );
            ui.checkbox(
                &mut self.edit_config.minimise_to_tray,
                "Minimise to tray on close",
            );
            ui.checkbox(
                &mut self.edit_config.start_minimised,
                "Start minimised to tray",
            );

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.config = self.edit_config.clone();
                    self.status_message = Some(match self.config.save() {
                        Ok(()) => "Saved.".into(),
                        Err(e) => format!("Save failed: {e}"),
                    });
                    if let Err(e) = self.hotkeys.re_register(&self.config) {
                        self.status_message = Some(format!("Hotkey rebind failed: {e}"));
                    }
                    let mut hs = self.hotkey_settings.lock().unwrap();
                    hs.api_url = self.config.api_url.clone();
                    hs.sound_enabled = self.config.sound_enabled;
                    hs.show_notifications = self.config.show_notifications;
                    hs.debounce_ms = self.config.debounce_ms;
                    hs.like_id = self.hotkeys.like_id();
                    hs.dislike_id = self.hotkeys.dislike_id();
                    drop(hs);
                    let _ = autostart::set_autostart(self.config.start_with_windows);
                }
                if ui.button("Cancel").clicked() {
                    self.edit_config = self.config.clone();
                    self.capturing_like = false;
                    self.capturing_dislike = false;
                }
                if ui.button("Hide to tray").clicked() {
                    self.set_window_visible(false);
                }
            });
            if let Some(msg) = &self.status_message {
                ui.label(msg);
            }
        });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
