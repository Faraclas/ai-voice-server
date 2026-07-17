pub fn parse_key(name: &str) -> Option<u16> {
    if name == "NONE" || name == "-1" { return None; }
    if let Ok(val) = name.parse::<u16>() { return Some(val); }
    match name {
        _ => None
    }
}
