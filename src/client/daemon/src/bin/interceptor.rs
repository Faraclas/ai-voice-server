use std::io::{self, Read, Write};
use std::net::UdpSocket;

const EV_KEY: u16 = 1;

fn main() {
    // Read the target keycode from arguments, defaulting to 97 (Right Ctrl)
    let args: Vec<String> = std::env::args().collect();
    let target_key: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(97); 

    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP socket");
    let target_addr = "127.0.0.1:9999";

    // A C input_event struct on 64-bit Linux is 24 bytes
    let mut buf = [0u8; 24];
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    loop {
        match stdin.read_exact(&mut buf) {
            Ok(_) => {
                let (_, rest) = buf.split_at(16); // skip timeval (16 bytes)
                let (type_bytes, rest) = rest.split_at(2);
                let (code_bytes, rest) = rest.split_at(2);
                let (value_bytes, _) = rest.split_at(4);

                let type_ = u16::from_ne_bytes(type_bytes.try_into().unwrap());
                let code = u16::from_ne_bytes(code_bytes.try_into().unwrap());
                let value = i32::from_ne_bytes(value_bytes.try_into().unwrap());

                let mut swallow = false;

                if type_ == EV_KEY && code == target_key {
                    if value == 1 {
                        let _ = socket.send_to(b"PRESS", target_addr);
                        swallow = true;
                    } else if value == 0 {
                        let _ = socket.send_to(b"RELEASE", target_addr);
                        swallow = true;
                    } else if value == 2 {
                        // Key repeat, swallow it but don't send anything
                        swallow = true;
                    }
                }

                if !swallow {
                    if let Err(_) = stdout.write_all(&buf) {
                        break;
                    }
                    let _ = stdout.flush();
                }
            }
            Err(_) => break, // EOF or error
        }
    }
}
