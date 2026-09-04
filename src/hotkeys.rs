use anyhow::{anyhow, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
pub use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::config::Config;

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    like_hotkey: HotKey,
    dislike_hotkey: HotKey,
}

impl Hotkeys {
    pub fn new(config: &Config) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let like_hotkey = parse_hotkey(&config.hotkey_like)?;
        let dislike_hotkey = parse_hotkey(&config.hotkey_dislike)?;
        manager.register(like_hotkey)?;
        manager.register(dislike_hotkey)?;
        Ok(Self {
            manager,
            like_hotkey,
            dislike_hotkey,
        })
    }

    pub fn like_id(&self) -> u32 {
        self.like_hotkey.id()
    }
    pub fn dislike_id(&self) -> u32 {
        self.dislike_hotkey.id()
    }

    pub fn re_register(&mut self, config: &Config) -> Result<()> {
        let _ = self.manager.unregister(self.like_hotkey);
        let _ = self.manager.unregister(self.dislike_hotkey);
        self.like_hotkey = parse_hotkey(&config.hotkey_like)?;
        self.dislike_hotkey = parse_hotkey(&config.hotkey_dislike)?;
        self.manager.register(self.like_hotkey)?;
        self.manager.register(self.dislike_hotkey)?;
        Ok(())
    }
}

pub fn parse_hotkey(spec: &str) -> Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in spec.split('+') {
        match part.trim().to_ascii_uppercase().as_str() {
            "ALT" => mods |= Modifiers::ALT,
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "SUPER" | "WIN" | "WINDOWS" => mods |= Modifiers::SUPER,
            other => code = Some(parse_code(other)?),
        }
    }
    let code = code.ok_or_else(|| anyhow!("hotkey '{spec}' has no key, only modifiers"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_code(key: &str) -> Result<Code> {
    use Code::*;
    Ok(match key {
        "A" => KeyA,
        "B" => KeyB,
        "C" => KeyC,
        "D" => KeyD,
        "E" => KeyE,
        "F" => KeyF,
        "G" => KeyG,
        "H" => KeyH,
        "I" => KeyI,
        "J" => KeyJ,
        "K" => KeyK,
        "L" => KeyL,
        "M" => KeyM,
        "N" => KeyN,
        "O" => KeyO,
        "P" => KeyP,
        "Q" => KeyQ,
        "R" => KeyR,
        "S" => KeyS,
        "T" => KeyT,
        "U" => KeyU,
        "V" => KeyV,
        "W" => KeyW,
        "X" => KeyX,
        "Y" => KeyY,
        "Z" => KeyZ,
        "0" | "NUM0" => Digit0,
        "1" | "NUM1" => Digit1,
        "2" | "NUM2" => Digit2,
        "3" | "NUM3" => Digit3,
        "4" | "NUM4" => Digit4,
        "5" | "NUM5" => Digit5,
        "6" | "NUM6" => Digit6,
        "7" | "NUM7" => Digit7,
        "8" | "NUM8" => Digit8,
        "9" | "NUM9" => Digit9,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "SPACE" => Space,
        "ENTER" | "RETURN" => Enter,
        "ESC" | "ESCAPE" => Escape,
        "TAB" => Tab,
        "UP" | "ARROWUP" => ArrowUp,
        "DOWN" | "ARROWDOWN" => ArrowDown,
        "LEFT" | "ARROWLEFT" => ArrowLeft,
        "RIGHT" | "ARROWRIGHT" => ArrowRight,
        "BACKSPACE" => Backspace,
        "DELETE" | "DEL" => Delete,
        "HOME" => Home,
        "END" => End,
        "PAGEUP" => PageUp,
        "PAGEDOWN" => PageDown,
        other => {
            return Err(anyhow!(
                "unrecognised key '{other}' — add it to hotkeys.rs::parse_code"
            ))
        }
    })
}
