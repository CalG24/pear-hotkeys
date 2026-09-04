static NOTIFY_SOUND: &[u8] = include_bytes!("../assets/notify.wav");

pub fn play_notification_sound() {
    std::thread::spawn(|| match rodio::OutputStream::try_default() {
        Ok((_stream, handle)) => {
            if let Ok(sink) = rodio::Sink::try_new(&handle) {
                let cursor = std::io::Cursor::new(NOTIFY_SOUND);
                match rodio::Decoder::new(cursor) {
                    Ok(source) => {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                    Err(e) => tracing::warn!("failed to decode notify.wav: {e}"),
                }
            }
        }
        Err(e) => tracing::warn!("no default audio output device: {e}"),
    });
}
