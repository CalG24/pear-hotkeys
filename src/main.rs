use notify_rust::Notification;
use reqwest::blocking::Client;
use rodio::{Decoder, OutputStream, Sink};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct LikeState {
    state: String,
}

struct HotkeyManager {
    client: Client,
    last_trigger: Instant,
    debounce_duration: Duration,
    alt_pressed: bool,
}

impl HotkeyManager {
    fn new() -> Self {
        Self {
            client: Client::new(),
            last_trigger: Instant::now() - Duration::from_secs(1),
            debounce_duration: Duration::from_millis(300),
            alt_pressed: false,
        }
    }

    fn handle_like(&mut self) {
        if self.is_debounced() {
            return;
        }

        self.last_trigger = Instant::now();

        thread::spawn(|| {
            if let Err(e) = Self::toggle_like() {
                eprintln!("Error toggling like: {}", e);
            }
        });
    }

    fn handle_dislike(&mut self) {
        if self.is_debounced() {
            return;
        }

        self.last_trigger = Instant::now();

        thread::spawn(|| {
            if let Err(e) = Self::toggle_dislike() {
                eprintln!("Error toggling dislike: {}", e);
            }
        });
    }

    fn is_debounced(&self) -> bool {
        self.last_trigger.elapsed() < self.debounce_duration
    }

    fn toggle_like() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new();
        
        // Get current state - FIXED: added .send()
        let state_response: LikeState = client
            .get("http://localhost:26538/api/v1/like-state")
            .send()?
            .json()?;

        // Toggle
        client.post("http://localhost:26538/api/v1/like").send()?;

        // Show notification
        let message = if state_response.state == "LIKE" {
            "Unliked 💔"
        } else {
            "Liked ❤️"
        };

        Notification::new()
            .summary(message)
            .appname("Pear Desktop")
            .show()?;

        // Play sound
        Self::play_sound();

        Ok(())
    }

    fn toggle_dislike() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new();
        
        // Get current state - FIXED: added .send()
        let state_response: LikeState = client
            .get("http://localhost:26538/api/v1/like-state")
            .send()?
            .json()?;

        // Toggle
        client.post("http://localhost:26538/api/v1/dislike").send()?;

        // Show notification
        let message = if state_response.state == "DISLIKE" {
            "Removed dislike 👍"
        } else {
            "Disliked 👎"
        };

        Notification::new()
            .summary(message)
            .appname("Pear Desktop")
            .show()?;

        // Play sound
        Self::play_sound();

        Ok(())
    }

    fn play_sound() {
        thread::spawn(|| {
            if let Ok((_, stream)) = OutputStream::try_default() {
                let sink = Sink::try_new(&stream).unwrap();
                
                // Try to load the sound file
                let sound_paths = [
                    "/usr/share/sounds/freedesktop/stereo/message.oga",
                    "/usr/share/sounds/freedesktop/stereo/message-new-instant.oga",
                ];

                for path in &sound_paths {
                    if let Ok(file) = File::open(path) {
                        if let Ok(source) = Decoder::new(BufReader::new(file)) {
                            sink.append(source);
                            sink.sleep_until_end();
                            break;
                        }
                    }
                }
            }
        });
    }
}

fn main() {
    println!("Pear Desktop Hotkeys v1.0.0");
    println!("Press Alt+L to like/unlike, Alt+D to dislike/undislike");
    println!("Press Ctrl+C to exit");

    let mut manager = HotkeyManager::new();

    // Listen for keyboard events
    rdev::listen(move |event| {
        use rdev::{EventType, Key};

        match event.event_type {
            // FIXED: Use Key::Alt instead of AltLeft/AltRight
            EventType::KeyPress(Key::Alt) => {
                manager.alt_pressed = true;
            }
            EventType::KeyRelease(Key::Alt) => {
                manager.alt_pressed = false;
            }
            EventType::KeyPress(Key::KeyL) => {
                if manager.alt_pressed {
                    manager.handle_like();
                }
            }
            EventType::KeyPress(Key::KeyD) => {
                if manager.alt_pressed {
                    manager.handle_dislike();
                }
            }
            _ => {}
        }
    })
    .expect("Failed to listen for keyboard events");
}
