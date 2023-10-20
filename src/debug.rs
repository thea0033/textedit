use std::fs::{self, read_to_string};

pub fn debug(s: &str) {
    let mut v = read_to_string("debug").unwrap();
    v.push('\n');
    v.push_str(s);
    fs::write("debug", v).unwrap();
}
