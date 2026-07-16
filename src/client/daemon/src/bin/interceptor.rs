use std::io::{self, Read, Write};
use std::net::UdpSocket;

const EV_KEY: u16 = 1;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Default to Left Ctrl (29) and Space (57)
    let modifier_key: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(29);
    let target_key: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(57);

    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP socket");
    let target_addr = "127.0.0.1:9999";

    let mut buf = [0u8; 24];
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mut mod_down = false;
    let mut active = false;

    loop {
        match stdin.read_exact(&mut buf) {
            Ok(_) => {
                let (_, rest) = buf.split_at(16);
                let (type_bytes, rest) = rest.split_at(2);
                let (code_bytes, rest) = rest.split_at(2);
                let (value_bytes, _) = rest.split_at(4);

                let type_ = u16::from_ne_bytes(type_bytes.try_into().unwrap());
                let code = u16::from_ne_bytes(code_bytes.try_into().unwrap());
                let value = i32::from_ne_bytes(value_bytes.try_into().unwrap());

                let mut swallow = false;

                if type_ == EV_KEY {
                    if code == modifier_key {
                        mod_down = value == 1 || value == 2;
                        // Never swallow the modifier, otherwise standard shortcuts break
                    } else if code == target_key {
                        if value == 1 {
                            if mod_down {
                                let _ = socket.send_to(b"PRESS", target_addr);
                                active = true;
                                swallow = true;
                            }
                        } else if value == 0 {
                            if active {
                                let _ = socket.send_to(b"RELEASE", target_addr);
                                active = false;
                                swallow = true;
                            }
                        } else if value == 2 {
                            if active {
                                swallow = true;
                            }
                        }
                    }
                }

                if !swallow {
                    if let Err(_) = stdout.write_all(&buf) {
                        break;
                    }
                    let _ = stdout.flush();
                }
            }
            Err(_) => break,
        }
    }
}
