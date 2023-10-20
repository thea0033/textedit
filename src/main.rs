#[allow(dead_code)]
mod ansi;
mod ansi_truncate;
#[allow(dead_code)]
mod debug;
mod keymap;
mod textbox;

use std::{
    collections::LinkedList,
    fs::{self},
};
use crossterm::{
    event::Event,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use grid_ui::grid::{Alignment, DividerStrategy, Frame, SplitStrategy};
use keymap::{KeyLevels, Mode};
use textbox::TextBox;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args();
    // discards the first arg (path to the program).
    let _ = args.next();
    // if there is a next argument...
    let map: KeyLevels =
        serde_json::from_str(&fs::read_to_string("map").unwrap_or("{[]}".to_string()))
            .expect("Invalid json scheme!");
    if let Some(val) = args.next() {
        open(&val, map)?;
    } else {
        // error message
        println!("Usage: cargo run <path> or program <path>");
    }
    Ok(())
}
pub const HEADER_SIZE: usize = 5;
pub struct State {
    pub mode: Mode,
    pub will_quit: bool,
}
impl State {
    pub fn new() -> State {
        State {
            mode: Mode::Command,
            will_quit: false,
        }
    }
}
fn open(p: &str, keymap: KeyLevels) -> std::io::Result<()> {
    // initializes the state
    let mut state = State::new();

    // gets the terminal's size
    let (x_max, y_max) = terminal::size()?;
    let mut f = Frame::new(0, 0, x_max as usize, y_max as usize);

    // reads the file to a string
    let mut file = std::fs::read_to_string(p)
        .unwrap_or_else(|_| String::new()) // reads the file into a string
        .lines() // splits into lines
        .map(|x| x.to_string()) // converts each line into a string
        .collect::<Vec<String>>(); // collects it into a vector

    // adds a newline to the file if it's empty.
    if file.len() == 0 {
        file.push(String::new());
    }

    // creates a textbox out of the file's output
    let mut text_box = TextBox::new(file, p.to_string());

    // enables raw mode for the terminal
    enable_raw_mode()?;

    // creates grid that represents the terminal
    let mut grid = f.next_frame();

    // splits the grid off into a headers section and initializes it into a draw process.
    let mut line_nums = grid
        .split(&SplitStrategy::new().max_x(HEADER_SIZE, Alignment::Minus))
        .expect("Terminal too small!") // if the terminal is too small, it will panic
        .into_process(DividerStrategy::Beginning); // lines are drawn from the top.

    // the remainder of the grid is the main section. Creates a draw process out of this.
    let mut d = grid.into_process(DividerStrategy::Beginning);
    // displays it for the first time.
    text_box.display(&mut d, &mut line_nums);
    'outer: while let Ok(val) = crossterm::event::read() {
        // If a key is pressed...
        if let Event::Key(val) = val {
            // handle this key.
            for i in keymap.map_keys(val, state.mode) {
                text_box.recv_key(i, &mut d, &mut line_nums, &mut state);
                if state.will_quit {
                    break 'outer;
                }
            }
        // If the screen is resized...
        } else if let Event::Resize(x, y) = val {
            // resizes the frame based on the new terminal size
            f.resize(0, 0, x.into(), y.into());
            // creates a grid based on that frame
            grid = f.next_frame();
            // splits the grid off once again
            line_nums = grid
                .split(&SplitStrategy::new().max_x(HEADER_SIZE, Alignment::Minus))
                .expect("Terminal too small!")
                .into_process(DividerStrategy::Beginning);
            // reinitializes the draw process.
            d = grid.into_process(DividerStrategy::Beginning);
        }
    }
    // disables raw mode for the terminal
    disable_raw_mode()?;
    Ok(())
}
