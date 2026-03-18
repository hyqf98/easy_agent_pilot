use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::{mpsc, Arc};
use std::thread;
use tauri::AppHandle;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_ADD, VK_BACK, VK_CAPITAL, VK_DECIMAL, VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE,
    VK_F1, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F2,
    VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9,
    VK_HOME, VK_INSERT, VK_LEFT, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK, VK_NUMPAD0, VK_NUMPAD1,
    VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9,
    VK_PAUSE, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SCROLL, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB,
    VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_INJECTED, MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ShortcutModifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
    super_key: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShortcutSpec {
    modifiers: ShortcutModifiers,
    key_code: u32,
}

#[derive(Debug)]
struct HookRuntime {
    app: AppHandle,
    shortcut: ShortcutSpec,
    pressed_modifiers: ShortcutModifiers,
    suppress_current_key: bool,
}

struct HookSession {
    thread_id: u32,
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
        pressed_modifiers: ShortcutModifiers::default(),
        suppress_current_key: false,
    }));

    let worker_runtime = runtime.clone();
    let join_handle = thread::spawn(move || {
        let mut msg: MSG = unsafe { zeroed() };
        unsafe {
            PeekMessageW(&mut msg, null_mut(), 0, 0, PM_NOREMOVE);
        }

        let thread_id = unsafe { GetCurrentThreadId() };
        *ACTIVE_RUNTIME.lock() = Some(worker_runtime);

        let hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), null_mut(), 0)
        };
        if hook.is_null() {
            *ACTIVE_RUNTIME.lock() = None;
            let _ = ready_tx.send(Err("WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string()));
            return;
        }

        let _ = ready_tx.send(Ok(thread_id));

        loop {
            let status = unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) };
            if status <= 0 {
                break;
            }

            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        unsafe {
            UnhookWindowsHookEx(hook);
        }
        *ACTIVE_RUNTIME.lock() = None;
    });

    let thread_id = ready_rx
        .recv()
        .map_err(|_| "WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string())??;

    *ACTIVE_SESSION.lock() = Some(HookSession {
        thread_id,
        join_handle,
    });

    Ok(())
}

pub fn unregister_shortcut() -> Result<(), String> {
    let session = ACTIVE_SESSION.lock().take();

    if let Some(session) = session {
        unsafe {
            PostThreadMessageW(session.thread_id, WM_QUIT, 0, 0);
        }

        session
            .join_handle
            .join()
            .map_err(|_| "WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string())?;
    }

    *ACTIVE_RUNTIME.lock() = None;
    Ok(())
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
            "CommandOrControl" | "Control" => modifiers.ctrl = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            "Super" => modifiers.super_key = true,
            value => {
                if key_code.is_some() {
                    return Err("WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string());
                }

                key_code = Some(parse_key_code(value)?);
            }
        }
    }

    let key_code = key_code.ok_or_else(|| "WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string())?;
    Ok(ShortcutSpec {
        modifiers,
        key_code,
    })
}

fn parse_key_code(token: &str) -> Result<u32, String> {
    if token.len() == 1 {
        let ch = token.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(ch.to_ascii_uppercase() as u32);
        }

        if ch.is_ascii_digit() {
            return Ok(ch as u32);
        }

        return match ch {
            '`' => Ok(0xC0),
            '-' => Ok(0xBD),
            '=' => Ok(0xBB),
            '[' => Ok(0xDB),
            ']' => Ok(0xDD),
            '\\' => Ok(0xDC),
            ';' => Ok(0xBA),
            '\'' => Ok(0xDE),
            ',' => Ok(0xBC),
            '.' => Ok(0xBE),
            '/' => Ok(0xBF),
            _ => Err("WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string()),
        };
    }

    match token {
        "Space" => Ok(VK_SPACE as u32),
        "Escape" => Ok(VK_ESCAPE as u32),
        "Enter" => Ok(VK_RETURN as u32),
        "Tab" => Ok(VK_TAB as u32),
        "Backspace" => Ok(VK_BACK as u32),
        "Delete" => Ok(VK_DELETE as u32),
        "End" => Ok(VK_END as u32),
        "Home" => Ok(VK_HOME as u32),
        "Insert" => Ok(VK_INSERT as u32),
        "PageDown" => Ok(VK_NEXT as u32),
        "PageUp" => Ok(VK_PRIOR as u32),
        "PrintScreen" => Ok(VK_SNAPSHOT as u32),
        "ScrollLock" => Ok(VK_SCROLL as u32),
        "CapsLock" => Ok(VK_CAPITAL as u32),
        "NumLock" => Ok(VK_NUMLOCK as u32),
        "Pause" => Ok(VK_PAUSE as u32),
        "Up" => Ok(VK_UP as u32),
        "Down" => Ok(VK_DOWN as u32),
        "Left" => Ok(VK_LEFT as u32),
        "Right" => Ok(VK_RIGHT as u32),
        "Numpad0" => Ok(VK_NUMPAD0 as u32),
        "Numpad1" => Ok(VK_NUMPAD1 as u32),
        "Numpad2" => Ok(VK_NUMPAD2 as u32),
        "Numpad3" => Ok(VK_NUMPAD3 as u32),
        "Numpad4" => Ok(VK_NUMPAD4 as u32),
        "Numpad5" => Ok(VK_NUMPAD5 as u32),
        "Numpad6" => Ok(VK_NUMPAD6 as u32),
        "Numpad7" => Ok(VK_NUMPAD7 as u32),
        "Numpad8" => Ok(VK_NUMPAD8 as u32),
        "Numpad9" => Ok(VK_NUMPAD9 as u32),
        "NumpadAdd" => Ok(VK_ADD as u32),
        "NumpadDecimal" => Ok(VK_DECIMAL as u32),
        "NumpadDivide" => Ok(VK_DIVIDE as u32),
        "NumpadEnter" => Ok(VK_RETURN as u32),
        "NumpadMultiply" => Ok(VK_MULTIPLY as u32),
        "NumpadSubtract" => Ok(VK_SUBTRACT as u32),
        "F1" => Ok(VK_F1 as u32),
        "F2" => Ok(VK_F2 as u32),
        "F3" => Ok(VK_F3 as u32),
        "F4" => Ok(VK_F4 as u32),
        "F5" => Ok(VK_F5 as u32),
        "F6" => Ok(VK_F6 as u32),
        "F7" => Ok(VK_F7 as u32),
        "F8" => Ok(VK_F8 as u32),
        "F9" => Ok(VK_F9 as u32),
        "F10" => Ok(VK_F10 as u32),
        "F11" => Ok(VK_F11 as u32),
        "F12" => Ok(VK_F12 as u32),
        "F13" => Ok(VK_F13 as u32),
        "F14" => Ok(VK_F14 as u32),
        "F15" => Ok(VK_F15 as u32),
        "F16" => Ok(VK_F16 as u32),
        "F17" => Ok(VK_F17 as u32),
        "F18" => Ok(VK_F18 as u32),
        "F19" => Ok(VK_F19 as u32),
        "F20" => Ok(VK_F20 as u32),
        "F21" => Ok(VK_F21 as u32),
        "F22" => Ok(VK_F22 as u32),
        "F23" => Ok(VK_F23 as u32),
        "F24" => Ok(VK_F24 as u32),
        _ => Err("WINDOWS_SHORTCUT_OVERRIDE_FAILED".to_string()),
    }
}

fn update_modifier_state(modifiers: &mut ShortcutModifiers, vk_code: u32, is_down: bool) {
    match vk_code {
        0x11 | 0xA2 | 0xA3 => modifiers.ctrl = is_down,
        0x12 | 0xA4 | 0xA5 => modifiers.alt = is_down,
        0x10 | 0xA0 | 0xA1 => modifiers.shift = is_down,
        0x5B | 0x5C => modifiers.super_key = is_down,
        _ => {}
    }
}

fn modifiers_match(actual: ShortcutModifiers, expected: ShortcutModifiers) -> bool {
    actual == expected
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if code != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(0 as HHOOK, code, w_param, l_param) };
    }

    let event = unsafe { *(l_param as *const KBDLLHOOKSTRUCT) };
    if (event.flags & LLKHF_INJECTED) != 0 {
        return unsafe { CallNextHookEx(0 as HHOOK, code, w_param, l_param) };
    }

    let runtime = {
        let guard = ACTIVE_RUNTIME.lock();
        guard.clone()
    };

    let Some(runtime) = runtime else {
        return unsafe { CallNextHookEx(0 as HHOOK, code, w_param, l_param) };
    };

    let mut runtime = runtime.lock();
    let is_key_down = matches!(w_param as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let is_key_up = matches!(w_param as u32, WM_KEYUP | WM_SYSKEYUP);

    if !is_key_down && !is_key_up {
        return unsafe { CallNextHookEx(0 as HHOOK, code, w_param, l_param) };
    }

    update_modifier_state(&mut runtime.pressed_modifiers, event.vkCode, is_key_down);

    let shortcut = runtime.shortcut;
    let is_target_key = event.vkCode == shortcut.key_code;

    if is_key_down && is_target_key {
        if runtime.suppress_current_key {
            return 1;
        }

        if modifiers_match(runtime.pressed_modifiers, shortcut.modifiers) {
            runtime.suppress_current_key = true;

            let app_handle = runtime.app.clone();
            let toggle_handle = app_handle.clone();
            let _ = app_handle.run_on_main_thread(move || {
                let _ = super::mini_panel::toggle_mini_panel(toggle_handle);
            });

            return 1;
        }
    }

    if is_key_up && is_target_key && runtime.suppress_current_key {
        runtime.suppress_current_key = false;
        return 1;
    }

    if runtime.suppress_current_key
        && !modifiers_match(runtime.pressed_modifiers, shortcut.modifiers)
    {
        runtime.suppress_current_key = false;
    }

    unsafe { CallNextHookEx(0 as HHOOK, code, w_param, l_param) }
}
