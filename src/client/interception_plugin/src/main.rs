use std::env;
use std::io::{self, Read, Write};
use std::net::UdpSocket;

// Constants from linux/input.h
const EV_KEY: u16 = 1;
const KEY_PRESS: i32 = 1;
const KEY_RELEASE: i32 = 0;

// Struct matching Linux input_event (64-bit platforms)
#[repr(C)]
struct InputEvent {
    tv_sec: i64,
    tv_usec: i64,
    type_: u16,
    code: u16,
    value: i32,
}

fn main() {
    // Parse arguments: [--key 57] [--modifier 29]
    // 57 = KEY_SPACE, 29 = KEY_LEFTCTRL
    let mut target_key: u16 = 57;
    let mut target_mod: Option<u16> = Some(29);
    let mut toggle_key: u16 = 57;
    let mut toggle_mod: Option<u16> = Some(97);

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                if i + 1 < args.len() {
                    target_key = args[i + 1].parse().unwrap_or(target_key);
                    i += 1;
                }
            }
            "--modifier" => {
                if i + 1 < args.len() {
                    let val: i32 = args[i + 1].parse().unwrap_or(-1);
                    target_mod = if val >= 0 { Some(val as u16) } else { None };
                    i += 1;
                }
            }
            "--toggle-key" => {
                if i + 1 < args.len() {
                    toggle_key = args[i + 1].parse().unwrap_or(toggle_key);
                    i += 1;
                }
            }
            "--toggle-modifier" => {
                if i + 1 < args.len() {
                    let val: i32 = args[i + 1].parse().unwrap_or(-1);
                    toggle_mod = if val >= 0 { Some(val as u16) } else { None };
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP socket");
    let daemon_addr = "127.0.0.1:9999";

    let mut buf = [0u8; 24]; // sizeof(input_event) on 64-bit Linux
    let mut modifier_pressed = false;
    let mut toggle_modifier_pressed = false;
    let mut recording = false;
    let mut key_was_swallowed = false;
    let mut waiting_for_mod_release = false;

    loop {
        if io::stdin().read_exact(&mut buf).is_err() {
            break;
        }

        let event: &InputEvent = unsafe { std::mem::transmute(&buf) };

        if event.type_ == EV_KEY {
            let mut is_modifier_event = false;

            // Track main modifier state
            if let Some(m) = target_mod {
                if event.code == m {
                    is_modifier_event = true;
                    if event.value == KEY_PRESS || event.value == 2 {
                        modifier_pressed = true;
                    } else if event.value == KEY_RELEASE {
                        modifier_pressed = false;
                        if waiting_for_mod_release {
                            let _ = socket.send_to(b"MODIFIER_UP", daemon_addr);
                            waiting_for_mod_release = false;
                        }
                    }
                }
            }

            // Track toggle modifier state
            if let Some(tm) = toggle_mod {
                if event.code == tm {
                    is_modifier_event = true;
                    if event.value == KEY_PRESS || event.value == 2 {
                        toggle_modifier_pressed = true;
                    } else if event.value == KEY_RELEASE {
                        toggle_modifier_pressed = false;
                    }
                }
            }

            if is_modifier_event {
                // Always pass through modifier keys so other shortcuts still work
                if io::stdout().write_all(&buf).is_err() { break; }
                if io::stdout().flush().is_err() { break; }
                continue;
            }

            // Check if it's the target key (Record)
            if event.code == target_key {
                let mod_ok = match target_mod {
                    Some(_) => modifier_pressed,
                    None => true,
                };

                // Check toggle hotkey (prevent overlapping keys if they share the same base key)
                let toggle_mod_ok = match toggle_mod {
                    Some(_) => toggle_modifier_pressed,
                    None => true,
                };

                if event.value == KEY_PRESS { // Key down
                    if toggle_mod_ok && event.code == toggle_key && !recording {
                        // Fire toggle mode!
                        let _ = socket.send_to(b"TOGGLE_MODE", daemon_addr);
                        key_was_swallowed = true;
                        continue;
                    } else if mod_ok {
                        if !recording {
                            // Toggle ON
                            let msg: &[u8] = b"PRESS";
                            let _ = socket.send_to(msg, daemon_addr);
                            recording = true;
                        } else {
                            // Toggle OFF
                            let msg: &[u8] = b"RELEASE";
                            let _ = socket.send_to(msg, daemon_addr);
                            recording = false;
                            
                            // Let the daemon know if we need to wait for the physical release of the modifier
                            if target_mod.is_some() {
                                if modifier_pressed {
                                    waiting_for_mod_release = true;
                                } else {
                                    let _ = socket.send_to(b"MODIFIER_UP", daemon_addr);
                                }
                            } else {
                                // No modifier required, so we consider it "up"
                                let _ = socket.send_to(b"MODIFIER_UP", daemon_addr);
                            }
                        }

                        key_was_swallowed = true;
                        continue;
                    }
                } else if event.value == KEY_RELEASE { // Key up
                    if key_was_swallowed {
                        // Swallow the release event so the OS doesn't see a stuck key
                        key_was_swallowed = false;
                        continue;
                    }
                } else if event.value == 2 { // Key repeat
                    if mod_ok || (toggle_mod_ok && event.code == toggle_key) {
                        continue;
                    }
                }
            } else if event.code == toggle_key {
                // In case toggle_key is DIFFERENT from target_key
                let toggle_mod_ok = match toggle_mod {
                    Some(_) => toggle_modifier_pressed,
                    None => true,
                };
                if event.value == KEY_PRESS {
                    if toggle_mod_ok && !recording {
                        let _ = socket.send_to(b"TOGGLE_MODE", daemon_addr);
                        key_was_swallowed = true;
                        continue;
                    }
                } else if event.value == KEY_RELEASE {
                    if key_was_swallowed {
                        key_was_swallowed = false;
                        continue;
                    }
                } else if event.value == 2 {
                    if toggle_mod_ok {
                        continue;
                    }
                }
            }
        }

        // Pass through all other events
        if io::stdout().write_all(&buf).is_err() {
            break;
        }
        if io::stdout().flush().is_err() {
            break;
        }
    }
}
