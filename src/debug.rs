use std::fs::{read_to_string, self};


pub fn debug(s: &str) {
    let mut v = read_to_string("debug").unwrap();
    v.push('\n');
    v.push_str(s);
    fs::write("debug", v).unwrap();
}