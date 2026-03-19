use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CallbackResult, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField, KeyCode,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ShortcutModifiers {
    control: bool,
    alt: bool,
    shift: bool,
    command: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShortcutSpec {
    modifiers: ShortcutModifiers,
    key_code: u16,
}

#[derive(Debug)]
struct HookRuntime {
    app: AppHandle,
    shortcut: ShortcutSpec,
    suppress_current_key: bool,
}

struct HookSession {
    run_loop: CFRunLoop,
    join_handle: thread::JoinHandle<()>,
}

static ACTIVE_RUNTIME: Lazy<Mutex<Option<Arc<Mutex<HookRuntime>>>>> =
    Lazy::new(|| Mutex::new(None));
static ACTIVE_SESSION: Lazy<Mutex<Option<HookSession>>> = Lazy::new(|| Mutex::new(None));

pub fn register_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    let shortcut = parse_shortcut(&shortcut)?;
    unregister_shortcut()?;

    let (ready_tx, ready_rx) = mpsc::channel();
    let runtime = Arc::new(Mutex::new(HookRuntime {
        app,
        shortcut,
        suppress_current_key: false,
    }));

    let worker_runtime = runtime.clone();
    let join_handle = thread::spawn(move || {
        let run_loop = CFRunLoop::get_current();
        *ACTIVE_RUNTIME.lock() = Some(worker_runtime.clone());
        let ready_tx_run = ready_tx.clone();

        let tap_result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown, CGEventType::KeyUp, CGEventType::FlagsChanged],
            move |_proxy, event_type, event| handle_registered_event(&worker_runtime, event_type, event),
            || {
                let _ = ready_tx_run.send(Ok(run_loop.clone()));
                CFRunLoop::run_current();
            },
        );

        if tap_result.is_err() {
            *ACTIVE_RUNTIME.lock() = None;
            let _ = ready_tx.send(Err("MACOS_SHORTCUT_OVERRIDE_PERMISSION_REQUIRED".to_string()));
            return;
        }

        *ACTIVE_RUNTIME.lock() = None;
    });

    let run_loop = ready_rx
        .recv()
        .map_err(|_| "MACOS_SHORTCUT_OVERRIDE_FAILED".to_string())??;

    *ACTIVE_SESSION.lock() = Some(HookSession {
        run_loop,
        join_handle,
    });

    Ok(())
}

pub fn unregister_shortcut() -> Result<(), String> {
    let session = ACTIVE_SESSION.lock().take();

    if let Some(session) = session {
        session.run_loop.stop();
        session
            .join_handle
            .join()
            .map_err(|_| "MACOS_SHORTCUT_OVERRIDE_FAILED".to_string())?;
    }

    *ACTIVE_RUNTIME.lock() = None;
    Ok(())
}

pub fn capture_shortcut_once(timeout_ms: Option<u64>) -> Result<String, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15_000).clamp(1_000, 60_000));
    let (result_tx, result_rx) = mpsc::channel::<Result<String, String>>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<CFRunLoop, String>>();

    let join_handle = thread::spawn(move || {
        let run_loop = CFRunLoop::get_current();
        let ready_tx_run = ready_tx.clone();
        let tap_result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            move |_proxy, event_type, event| handle_capture_event(&result_tx, event_type, event),
            || {
                let _ = ready_tx_run.send(Ok(run_loop.clone()));
                CFRunLoop::run_current();
            },
        );

        if tap_result.is_err() {
            let _ = ready_tx.send(Err("MACOS_SHORTCUT_OVERRIDE_PERMISSION_REQUIRED".to_string()));
        }
    });

    let run_loop = ready_rx
        .recv()
        .map_err(|_| "MACOS_SHORTCUT_CAPTURE_FAILED".to_string())??;

    let result = match result_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => {
            run_loop.stop();
            join_handle
                .join()
                .map_err(|_| "MACOS_SHORTCUT_CAPTURE_FAILED".to_string())?;
            return Err("MACOS_SHORTCUT_CAPTURE_TIMEOUT".to_string());
        }
    };

    run_loop.stop();
    join_handle
        .join()
        .map_err(|_| "MACOS_SHORTCUT_CAPTURE_FAILED".to_string())?;

    result
}

fn handle_registered_event(
    runtime: &Arc<Mutex<HookRuntime>>,
    event_type: CGEventType,
    event: &core_graphics::event::CGEvent,
) -> CallbackResult {
    let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
    let flags = event.get_flags();

    let mut runtime = runtime.lock();
    let shortcut = runtime.shortcut;
    let matches_modifiers = modifiers_match(flags, shortcut.modifiers);
    let is_target_key = key_code == shortcut.key_code;

    if matches!(event_type, CGEventType::KeyDown) && is_target_key && matches_modifiers {
        if runtime.suppress_current_key {
            return CallbackResult::Drop;
        }

        runtime.suppress_current_key = true;
        let app_handle = runtime.app.clone();
        let toggle_handle = app_handle.clone();
        let _ = app_handle.run_on_main_thread(move || {
            let _ = super::mini_panel::toggle_mini_panel(toggle_handle);
        });
        return CallbackResult::Drop;
    }

    if matches!(event_type, CGEventType::KeyUp) && is_target_key && runtime.suppress_current_key {
        runtime.suppress_current_key = false;
        return CallbackResult::Drop;
    }

    if runtime.suppress_current_key && !matches_modifiers {
        runtime.suppress_current_key = false;
    }

    CallbackResult::Keep
}

fn handle_capture_event(
    result_tx: &mpsc::Sender<Result<String, String>>,
    event_type: CGEventType,
    event: &core_graphics::event::CGEvent,
) -> CallbackResult {
    if !matches!(event_type, CGEventType::KeyDown) {
        return CallbackResult::Keep;
    }

    let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
    let flags = event.get_flags();

    if key_code == KeyCode::ESCAPE && active_modifiers(flags) == ShortcutModifiers::default() {
        let _ = result_tx.send(Err("MACOS_SHORTCUT_CAPTURE_CANCELLED".to_string()));
        CFRunLoop::get_current().stop();
        return CallbackResult::Drop;
    }

    let Some(shortcut) = shortcut_from_event(flags, key_code) else {
        return CallbackResult::Keep;
    };

    let _ = result_tx.send(Ok(shortcut));
    CFRunLoop::get_current().stop();
    CallbackResult::Drop
}

fn shortcut_from_event(flags: CGEventFlags, key_code: u16) -> Option<String> {
    let key_token = key_code_to_token(key_code)?;
    let modifiers = active_modifiers(flags);
    let mut tokens = Vec::new();

    if modifiers.command {
        tokens.push("CommandOrControl");
    }
    if modifiers.control {
        tokens.push("Control");
    }
    if modifiers.alt {
        tokens.push("Alt");
    }
    if modifiers.shift {
        tokens.push("Shift");
    }

    tokens.push(key_token);
    Some(tokens.join("+"))
}

fn active_modifiers(flags: CGEventFlags) -> ShortcutModifiers {
    ShortcutModifiers {
        control: flags.contains(CGEventFlags::CGEventFlagControl),
        alt: flags.contains(CGEventFlags::CGEventFlagAlternate),
        shift: flags.contains(CGEventFlags::CGEventFlagShift),
        command: flags.contains(CGEventFlags::CGEventFlagCommand),
    }
}

fn modifiers_match(flags: CGEventFlags, expected: ShortcutModifiers) -> bool {
    active_modifiers(flags) == expected
}

fn parse_shortcut(shortcut: &str) -> Result<ShortcutSpec, String> {
    let mut modifiers = ShortcutModifiers::default();
    let mut key_code = None;

    for token in shortcut
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        match token {
            "CommandOrControl" | "Super" => modifiers.command = true,
            "Control" => modifiers.control = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            value => {
                if key_code.is_some() {
                    return Err("MACOS_SHORTCUT_OVERRIDE_FAILED".to_string());
                }

                key_code = Some(parse_key_code(value)?);
            }
        }
    }

    let key_code = key_code.ok_or_else(|| "MACOS_SHORTCUT_OVERRIDE_FAILED".to_string())?;
    Ok(ShortcutSpec {
        modifiers,
        key_code,
    })
}

fn parse_key_code(token: &str) -> Result<u16, String> {
    if token.len() == 1 {
        let ch = token.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(match ch.to_ascii_uppercase() {
                'A' => KeyCode::ANSI_A,
                'B' => KeyCode::ANSI_B,
                'C' => KeyCode::ANSI_C,
                'D' => KeyCode::ANSI_D,
                'E' => KeyCode::ANSI_E,
                'F' => KeyCode::ANSI_F,
                'G' => KeyCode::ANSI_G,
                'H' => KeyCode::ANSI_H,
                'I' => KeyCode::ANSI_I,
                'J' => KeyCode::ANSI_J,
                'K' => KeyCode::ANSI_K,
                'L' => KeyCode::ANSI_L,
                'M' => KeyCode::ANSI_M,
                'N' => KeyCode::ANSI_N,
                'O' => KeyCode::ANSI_O,
                'P' => KeyCode::ANSI_P,
                'Q' => KeyCode::ANSI_Q,
                'R' => KeyCode::ANSI_R,
                'S' => KeyCode::ANSI_S,
                'T' => KeyCode::ANSI_T,
                'U' => KeyCode::ANSI_U,
                'V' => KeyCode::ANSI_V,
                'W' => KeyCode::ANSI_W,
                'X' => KeyCode::ANSI_X,
                'Y' => KeyCode::ANSI_Y,
                'Z' => KeyCode::ANSI_Z,
                _ => return Err("MACOS_SHORTCUT_OVERRIDE_FAILED".to_string()),
            });
        }

        if ch.is_ascii_digit() {
            return Ok(match ch {
                '0' => KeyCode::ANSI_0,
                '1' => KeyCode::ANSI_1,
                '2' => KeyCode::ANSI_2,
                '3' => KeyCode::ANSI_3,
                '4' => KeyCode::ANSI_4,
                '5' => KeyCode::ANSI_5,
                '6' => KeyCode::ANSI_6,
                '7' => KeyCode::ANSI_7,
                '8' => KeyCode::ANSI_8,
                '9' => KeyCode::ANSI_9,
                _ => return Err("MACOS_SHORTCUT_OVERRIDE_FAILED".to_string()),
            });
        }

        return Ok(match ch {
            '`' => KeyCode::ANSI_GRAVE,
            '-' => KeyCode::ANSI_MINUS,
            '=' => KeyCode::ANSI_EQUAL,
            '[' => KeyCode::ANSI_LEFT_BRACKET,
            ']' => KeyCode::ANSI_RIGHT_BRACKET,
            '\\' => KeyCode::ANSI_BACKSLASH,
            ';' => KeyCode::ANSI_SEMICOLON,
            '\'' => KeyCode::ANSI_QUOTE,
            ',' => KeyCode::ANSI_COMMA,
            '.' => KeyCode::ANSI_PERIOD,
            '/' => KeyCode::ANSI_SLASH,
            _ => return Err("MACOS_SHORTCUT_OVERRIDE_FAILED".to_string()),
        });
    }

    Ok(match token {
        "Space" => KeyCode::SPACE,
        "Escape" => KeyCode::ESCAPE,
        "Enter" => KeyCode::RETURN,
        "Tab" => KeyCode::TAB,
        "Backspace" => KeyCode::DELETE,
        "Delete" => KeyCode::FORWARD_DELETE,
        "End" => KeyCode::END,
        "Home" => KeyCode::HOME,
        "PageDown" => KeyCode::PAGE_DOWN,
        "PageUp" => KeyCode::PAGE_UP,
        "Up" => KeyCode::UP_ARROW,
        "Down" => KeyCode::DOWN_ARROW,
        "Left" => KeyCode::LEFT_ARROW,
        "Right" => KeyCode::RIGHT_ARROW,
        "Numpad0" => KeyCode::ANSI_KEYPAD_0,
        "Numpad1" => KeyCode::ANSI_KEYPAD_1,
        "Numpad2" => KeyCode::ANSI_KEYPAD_2,
        "Numpad3" => KeyCode::ANSI_KEYPAD_3,
        "Numpad4" => KeyCode::ANSI_KEYPAD_4,
        "Numpad5" => KeyCode::ANSI_KEYPAD_5,
        "Numpad6" => KeyCode::ANSI_KEYPAD_6,
        "Numpad7" => KeyCode::ANSI_KEYPAD_7,
        "Numpad8" => KeyCode::ANSI_KEYPAD_8,
        "Numpad9" => KeyCode::ANSI_KEYPAD_9,
        "NumpadAdd" => KeyCode::ANSI_KEYPAD_PLUS,
        "NumpadDecimal" => KeyCode::ANSI_KEYPAD_DECIMAL,
        "NumpadDivide" => KeyCode::ANSI_KEYPAD_DIVIDE,
        "NumpadEnter" => KeyCode::ANSI_KEYPAD_ENTER,
        "NumpadEqual" => KeyCode::ANSI_KEYPAD_EQUAL,
        "NumpadMultiply" => KeyCode::ANSI_KEYPAD_MULTIPLY,
        "NumpadSubtract" => KeyCode::ANSI_KEYPAD_MINUS,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "F13" => KeyCode::F13,
        "F14" => KeyCode::F14,
        "F15" => KeyCode::F15,
        "F16" => KeyCode::F16,
        "F17" => KeyCode::F17,
        "F18" => KeyCode::F18,
        "F19" => KeyCode::F19,
        "F20" => KeyCode::F20,
        _ => return Err("MACOS_SHORTCUT_OVERRIDE_FAILED".to_string()),
    })
}

fn key_code_to_token(key_code: u16) -> Option<&'static str> {
    Some(match key_code {
        KeyCode::ANSI_A => "A",
        KeyCode::ANSI_B => "B",
        KeyCode::ANSI_C => "C",
        KeyCode::ANSI_D => "D",
        KeyCode::ANSI_E => "E",
        KeyCode::ANSI_F => "F",
        KeyCode::ANSI_G => "G",
        KeyCode::ANSI_H => "H",
        KeyCode::ANSI_I => "I",
        KeyCode::ANSI_J => "J",
        KeyCode::ANSI_K => "K",
        KeyCode::ANSI_L => "L",
        KeyCode::ANSI_M => "M",
        KeyCode::ANSI_N => "N",
        KeyCode::ANSI_O => "O",
        KeyCode::ANSI_P => "P",
        KeyCode::ANSI_Q => "Q",
        KeyCode::ANSI_R => "R",
        KeyCode::ANSI_S => "S",
        KeyCode::ANSI_T => "T",
        KeyCode::ANSI_U => "U",
        KeyCode::ANSI_V => "V",
        KeyCode::ANSI_W => "W",
        KeyCode::ANSI_X => "X",
        KeyCode::ANSI_Y => "Y",
        KeyCode::ANSI_Z => "Z",
        KeyCode::ANSI_0 => "0",
        KeyCode::ANSI_1 => "1",
        KeyCode::ANSI_2 => "2",
        KeyCode::ANSI_3 => "3",
        KeyCode::ANSI_4 => "4",
        KeyCode::ANSI_5 => "5",
        KeyCode::ANSI_6 => "6",
        KeyCode::ANSI_7 => "7",
        KeyCode::ANSI_8 => "8",
        KeyCode::ANSI_9 => "9",
        KeyCode::ANSI_GRAVE => "`",
        KeyCode::ANSI_MINUS => "-",
        KeyCode::ANSI_EQUAL => "=",
        KeyCode::ANSI_LEFT_BRACKET => "[",
        KeyCode::ANSI_RIGHT_BRACKET => "]",
        KeyCode::ANSI_BACKSLASH => "\\",
        KeyCode::ANSI_SEMICOLON => ";",
        KeyCode::ANSI_QUOTE => "'",
        KeyCode::ANSI_COMMA => ",",
        KeyCode::ANSI_PERIOD => ".",
        KeyCode::ANSI_SLASH => "/",
        KeyCode::SPACE => "Space",
        KeyCode::ESCAPE => "Escape",
        KeyCode::RETURN => "Enter",
        KeyCode::TAB => "Tab",
        KeyCode::DELETE => "Backspace",
        KeyCode::FORWARD_DELETE => "Delete",
        KeyCode::END => "End",
        KeyCode::HOME => "Home",
        KeyCode::PAGE_DOWN => "PageDown",
        KeyCode::PAGE_UP => "PageUp",
        KeyCode::UP_ARROW => "Up",
        KeyCode::DOWN_ARROW => "Down",
        KeyCode::LEFT_ARROW => "Left",
        KeyCode::RIGHT_ARROW => "Right",
        KeyCode::ANSI_KEYPAD_0 => "Numpad0",
        KeyCode::ANSI_KEYPAD_1 => "Numpad1",
        KeyCode::ANSI_KEYPAD_2 => "Numpad2",
        KeyCode::ANSI_KEYPAD_3 => "Numpad3",
        KeyCode::ANSI_KEYPAD_4 => "Numpad4",
        KeyCode::ANSI_KEYPAD_5 => "Numpad5",
        KeyCode::ANSI_KEYPAD_6 => "Numpad6",
        KeyCode::ANSI_KEYPAD_7 => "Numpad7",
        KeyCode::ANSI_KEYPAD_8 => "Numpad8",
        KeyCode::ANSI_KEYPAD_9 => "Numpad9",
        KeyCode::ANSI_KEYPAD_PLUS => "NumpadAdd",
        KeyCode::ANSI_KEYPAD_DECIMAL => "NumpadDecimal",
        KeyCode::ANSI_KEYPAD_DIVIDE => "NumpadDivide",
        KeyCode::ANSI_KEYPAD_ENTER => "NumpadEnter",
        KeyCode::ANSI_KEYPAD_EQUAL => "NumpadEqual",
        KeyCode::ANSI_KEYPAD_MULTIPLY => "NumpadMultiply",
        KeyCode::ANSI_KEYPAD_MINUS => "NumpadSubtract",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        KeyCode::F13 => "F13",
        KeyCode::F14 => "F14",
        KeyCode::F15 => "F15",
        KeyCode::F16 => "F16",
        KeyCode::F17 => "F17",
        KeyCode::F18 => "F18",
        KeyCode::F19 => "F19",
        KeyCode::F20 => "F20",
        _ => return None,
    })
}
