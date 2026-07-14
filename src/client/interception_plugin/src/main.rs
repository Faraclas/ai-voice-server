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
            _ => {}
        }
        i += 1;
    }

    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP socket");
    let daemon_addr = "127.0.0.1:9999";

    let mut buf = [0u8; 24]; // sizeof(input_event) on 64-bit Linux
    let mut modifier_pressed = false;
    let mut is_active = false;

    loop {
        if io::stdin().read_exact(&mut buf).is_err() {
            break;
        }

        let event: &InputEvent = unsafe { std::mem::transmute(&buf) };

        if event.type_ == EV_KEY {
            // Track modifier state if a modifier is configured
            if let Some(m) = target_mod {
                if event.code == m {
                    if event.value == KEY_PRESS {
                        modifier_pressed = true;
                    } else if event.value == KEY_RELEASE {
                        modifier_pressed = false;
                    }
                    // Always pass through the modifier key so other shortcuts still work
                    if io::stdout().write_all(&buf).is_err() {
                        break;
                    }
                    continue;
                }
            }

            // Check if it's our target key
            if event.code == target_key {
                let mod_ok = match target_mod {
                    Some(_) => modifier_pressed,
                    None => true,
                };

                // Trigger press if modifier is ok
                // Trigger release if we are currently active (even if modifier was released early)
                let should_trigger = (event.value == KEY_PRESS && mod_ok) || (event.value == KEY_RELEASE && is_active);

                if should_trigger {
                    if event.value == KEY_PRESS {
                        is_active = true;
                        let msg: &[u8] = b"PRESS";
                        let _ = socket.send_to(msg, daemon_addr);
                    } else if event.value == KEY_RELEASE {
                        is_active = false;
                        let msg: &[u8] = b"RELEASE";
                        let _ = socket.send_to(msg, daemon_addr);
                    }
                    
                    // Consume the target key (do not write to stdout)
                    // This prevents the OS from receiving 'SPACE' when CTRL is held
                    continue;
                }
            }
        }

        // Pass through all other events
        if io::stdout().write_all(&buf).is_err() {
            break;
        }
    }
}
