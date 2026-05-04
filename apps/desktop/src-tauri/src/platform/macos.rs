use anyhow::Result;
use arboard::Clipboard;
use rdev::{listen, simulate, Button, EventType, Key};
use std::sync::OnceLock;
use std::thread;

pub fn start_input_capture() -> Result<()> {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        thread::spawn(|| {
            let _ = listen(|_event| {
                // TODO: Route captured input events into the network pipeline.
            });
        });
    });
    Ok(())
}

pub fn inject_mouse_move(dx: i32, dy: i32) -> Result<()> {
    simulate(&EventType::MouseMove {
        x: dx as f64,
        y: dy as f64,
    })?;
    Ok(())
}

pub fn inject_key(code: u32, down: bool) -> Result<()> {
    if down {
        simulate(&EventType::KeyPress(Key::Unknown(code)))?;
    } else {
        simulate(&EventType::KeyRelease(Key::Unknown(code)))?;
    }
    Ok(())
}

pub fn inject_mouse_button(button: u8, down: bool) -> Result<()> {
    let mapped = match button {
        1 => Button::Left,
        2 => Button::Right,
        _ => Button::Middle,
    };
    if down {
        simulate(&EventType::ButtonPress(mapped))?;
    } else {
        simulate(&EventType::ButtonRelease(mapped))?;
    }
    Ok(())
}

pub fn set_clipboard_text(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}
