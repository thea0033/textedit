
#[allow(dead_code)]
mod debug;
#[allow(dead_code)]
mod ansi;
mod keymap;

use std::{collections::{LinkedList}, fs::{write, self}, io::stdout, fmt::Display};

use crossterm::{Result, event::{Event, KeyCode, KeyEvent, KeyModifiers}, terminal::{self, disable_raw_mode, enable_raw_mode}};
use grid_ui::{crossterm::CrosstermHandler, grid::{Alignment, DividerStrategy, Frame, SplitStrategy}, process::DrawProcess, trim::{TrimStrategy, TrimmedText}};
use keymap::{KeyLevels, Mode};
use unicode_segmentation::UnicodeSegmentation;


fn main() -> Result<()> {
    let mut args = std::env::args();
    // discards the first arg (path to the program). 
    let _ = args.next();
    // if there is a next argument... 
    let map: KeyLevels = serde_json::from_str(&fs::read_to_string("map").unwrap_or("{[]}".to_string())).expect("Invalid json scheme!");
    if let Some(val) = args.next() {
        open(&val, map)?;
    } else {
        // error message
        println!("Usage: cargo run <path> or program <path>");
    }
    Ok(())
}
// fn map_test() {
//     std::fs::write("map", serde_json::to_string_pretty(&keymap::KeyLevels {
//         levels: vec![
//             KeyLevel { recurse: vec![KeyMap { 
//                 pattern: KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL), result: Vec::new(), mode_req: keymap::ModeReq::Any 
//             }], fall: vec![] }
//         ]
//     }).unwrap()).unwrap();
// }
pub const HEADER_SIZE:usize = 5;
pub struct State {
    pub mode: Mode,
    pub will_quit: bool,
}
impl State {
    pub fn new() -> State {
        State { mode: Mode::Command, will_quit: false }
    }
}
fn open(p: &str, keymap: KeyLevels) -> std::io::Result<()> {
    // initializes the state
    let mut state = State::new();
    // gets the terminal's size
    let (x_max, y_max) = terminal::size()?;
    let mut f = Frame::new(0, 0, x_max as usize, y_max as usize);
    // reads the file to a string
    let mut file = std::fs::read_to_string(p)?.lines().map(|x| x.to_string()).collect::<Vec<String>>();
    // adds a newline to the file
    file.push(String::new());
    // creates a textbox out of the file's output
    let mut text_box = TextBox::new(file, p.to_string());
    // enables raw mode for the terminal
    enable_raw_mode()?;
    // creates grid that represents the terminal
    let mut grid = f.next_frame();
    // splits the grid off into a headers section and initializes it into a draw process. 
    let mut headers = grid.split(&SplitStrategy::new().max_x(HEADER_SIZE, Alignment::Minus)).expect("Terminal too small!").into_process(DividerStrategy::Beginning);
    // the remainder of the grid is the main section. Creates a draw process out of this. 
    let mut d = grid.into_process(DividerStrategy::Beginning);
    // displays it for the first time. 
    text_box.display(&mut d, &mut headers);
    'outer: while let Ok(val) = crossterm::event::read() {
        // If a key is pressed... 
        if let Event::Key(val) = val {
            // handle this key. 
            for i in keymap.map_keys(val, state.mode) {
                text_box.recv_key(i, &mut d, &mut headers, &mut state);
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
            headers = grid.split(&SplitStrategy::new().max_x(HEADER_SIZE, Alignment::Minus)).expect("Terminal too small!").into_process(DividerStrategy::Beginning);
            // reinitializes the draw process. 
            d = grid.into_process(DividerStrategy::Beginning);
        }
    }
    // disables raw mode for the terminal
    disable_raw_mode()?;
    Ok(())
}
#[derive(Debug, Clone)]
pub struct TwoLinkedList<T> {
    left: LinkedList<T>,
    right: LinkedList<T>
}
impl<T> TwoLinkedList<T> {
    pub fn new() -> TwoLinkedList<T> {
        TwoLinkedList { left: LinkedList::new(), right: LinkedList::new() }
    }
    pub fn with_right<I>(v: I) -> TwoLinkedList<T> where I: DoubleEndedIterator<Item = T> {
        TwoLinkedList {
            left: LinkedList::new(),
            right: v.collect(),
        }
    }
    pub fn insert(&mut self, i: T) -> usize {
        self.left.push_back(i);
        self.left.len()
    }
    pub fn y_right(&mut self) -> Option<usize> {
        if self.right.len() > 1 {
            let val = self.right.pop_front().expect("Safe unwrap");
            self.left.push_back(val);
            Some(self.left.len())
        } else {
            None
        }
    }
    pub fn x_right(&mut self) -> Option<usize> {
        if let Some(val) = self.right.pop_front() {
            self.left.push_back(val);
            Some(self.left.len())
        } else {
            None
        }
    }
    pub fn left(&mut self) -> Option<usize> {
        if let Some(val) = self.left.pop_back() {
            self.right.push_front(val);
            Some(self.left.len())
        } else {
            None
        }
    }
    pub fn from_left(&mut self) -> usize {
        self.left.append(&mut self.right);
        self.left.len()
    }
    pub fn from_right(&mut self) -> usize {
        self.left.append(&mut self.right);
        std::mem::swap(&mut self.left, &mut self.right);
        self.left.len()
    }
    pub fn from_pos(&mut self, p: usize) {
        self.from_left();
        while self.left.len() > p {
            if let None = self.left() {
                break;
            }
        }
    }
    pub fn enter(&mut self) -> TwoLinkedList<T> {
        TwoLinkedList {
            left: LinkedList::new(),
            right: std::mem::replace(&mut self.right, LinkedList::new()),
        }
    }
    pub fn backspace(&mut self) -> Option<usize>  {
        self.left.pop_back()?;
        Some(self.left.len())
    }
    pub fn splice_from_left(&mut self, mut other: TwoLinkedList<T>) {
        self.left.append(&mut other.left);
        self.left.append(&mut other.right);
    }
    pub fn splice_from_right(&mut self, mut other: TwoLinkedList<T>) {
        self.right.append(&mut other.left);
        self.right.append(&mut other.right);
        
    }
    pub fn delete(&mut self) -> Option<usize>  {
        self.right.pop_front()?;
        Some(self.left.len())
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.left.iter().chain(self.right.iter())
    }
    /// TODO: FIX
    pub fn get_y(&mut self) -> &mut T {
        self.right.front_mut().unwrap() // There CANNOT be a call when this is at the end
    }
}
pub struct TextBox {
    x_pos: usize,
    y_pos: usize,
    contents: TwoLinkedList<TwoLinkedList<char>>,
    path: String,
}
impl TextBox {
    pub fn new(lines: Vec<String>, path: String) -> TextBox {
        TextBox {
            x_pos: 0,
            y_pos: 0,
            contents: TwoLinkedList::with_right(lines.into_iter().map(|x| TwoLinkedList::with_right(x.chars()))),
            path,
        }
    }
    // Handles the key press. 
    pub fn recv_key(&mut self, k: KeyEvent, d: &mut DrawProcess, headers: &mut DrawProcess, state: &mut State) {
        let KeyEvent { code, modifiers } = k; 
        match code {
            // deletes the previous character if there is one. Merges two lines if needed. 
            KeyCode::Backspace => {
                if let Some(val) = self.contents.get_y().backspace() {
                    self.x_pos = val;
                } else {
                    let mut current = self.contents.right.pop_front().expect("BUFFER ABSENT!");
                    if let Some(prev) = self.contents.left.pop_back() {
                        current.splice_from_left(prev);
                    }
                    self.contents.right.push_front(current);
                }
            },
            // splits the current line into two lines. 
            KeyCode::Enter => {
                let mut v2 = self.contents.right.pop_front().expect("BUFFER ABSENT!");
                let new_line = v2.enter();
                self.contents.right.push_front(new_line);
                self.contents.left.push_back(v2);
                self.y_pos = self.contents.left.len();
            },
            // moves the cursor leftwards, or to the end of the previous line. 
            KeyCode::Left => {
                if let Some(val) = self.contents.get_y().left() {
                    self.x_pos = val;
                } else {
                    if let Some(val) = self.contents.left() {
                        self.y_pos = val;
                        self.x_pos = self.contents.get_y().from_left();
                    }
                }
            },
            // moves the cursor rightwards, or to the beginning of the next line. 
            KeyCode::Right => {
                if let Some(val) = self.contents.get_y().x_right() {
                    self.x_pos = val;
                } else {
                    if let Some(val) = self.contents.y_right() {
                        self.y_pos = val;
                        self.x_pos = self.contents.get_y().from_right();
                    }
                }
            },
            // moves the cursor upwards, or to the end of the last line (if on the last line)
            KeyCode::Up => {
                if let Some(val) = self.contents.left() {
                    self.contents.get_y().from_pos(self.x_pos);
                    self.y_pos = val;
                } else {
                    self.contents.get_y().from_right();
                }
            },
            // moves the cursor downwards, or to the beginning of the first line (if on the first line). 
            KeyCode::Down => {
                if let Some(val) = self.contents.y_right() {
                    self.contents.get_y().from_pos(self.x_pos);
                    self.y_pos = val;
                } else {
                    self.x_pos = self.contents.get_y().from_left();
                }
            },
            // Inserts a tab character. 
            KeyCode::Tab => {
                self.contents.get_y().insert('\t');
            },
            // Deletes the next character if there is one. Merges two lines if needed. 
            KeyCode::Delete => {
                if let Some(val) = self.contents.get_y().delete() {
                    self.x_pos = val;
                } else {
                    let mut current = self.contents.right.pop_front().expect("BUFFER ABSENT!");
                    if let Some(next) = self.contents.right.pop_front() {
                        current.splice_from_right(next);
                    }
                    self.contents.right.push_front(current);
                }
            },
            // Either executes a control sequence or adds a key. 
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.ctrl_keys(c, modifiers, state);
                } else {
                    self.contents.get_y().insert(c);
                }

            },
            // Escape currently does nothing, but might do something. 
            KeyCode::Esc => {},
            // No other key presses currently do anything. 
            _ => {}
        }
        self.display(d, headers);
    }
    // Executes a control sequence, and returns true if the program should end. 
    pub fn ctrl_keys(&mut self, c: char, _m: KeyModifiers, state: &mut State) {
        match c {
            // ctrl+c will copy text in the future
            'c' | 'C' => {

            },
            // If ctrl+q is pressed, the program should end.  
            'q' | 'Q' => {
                state.will_quit = true;
            },
            // ctrl+v will paste text in the future
            'v' | 'V' => {},
            // ctrl+x will cut text in the future
            'x' | 'X' => {},
            // ctrl+s saves the file
            's' | 'S' => {
                if let Err(_) = write(
                    &self.path, self.contents.iter() // iterates through the lines
                    .map(|x| x.iter().collect::<String>()) // collects each line into a string
                    .collect::<Vec<String>>() // collects each line into a vector of lines
                    .join("\n")) { // joins the lines with a newline character
                        // attempts to write the resulting data. If the write fails, prints a message. 
                    println!("Failed to save!");
                }
            }
            _ => {}
        }
    }
    // Calculates the position where the display starts printing. 
    pub fn calculate_start(&mut self, height: usize) -> usize {
        let current_line = self.contents.left.len();
        let half_pos = height / 2;
        let total_length = self.contents.left.len() + self.contents.right.len();
        if current_line < half_pos || total_length <= height {
            0
        } else if current_line >= total_length - half_pos {
            total_length - height
        } else {
            current_line - half_pos
        }
    }
    pub fn display(&mut self, d: &mut DrawProcess, headers: &mut DrawProcess) {
        let start = self.calculate_start(d.height());
        // the length of the left side of the twolinkedlist. This is used for the cursor. 
        let left_len = self.contents.left.len();
        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut main_strategy = AnsiTruncate::new(ansi::RESET, ansi::RESET);
        let mut current_line_strategy = AnsiTruncate::new(ansi::SELECTED, ansi::RESET);
        // makes a copy of the contents variable. 
        let contents = self.contents.clone();
        // chains the contents together. 
        let combined = contents.left.into_iter().chain(contents.right.into_iter());
        // enumerates through the contents. 
        for (i, line) in combined.enumerate().skip(start) {
            // the left portion of the line
            let left = line.left.into_iter();
            // the right portion of the line
            let mut right = line.right.into_iter();
            // if i is where the cursor is at... 
            if i == left_len {
                // 
                let current = right.next().unwrap_or(' ');
                let collected = format!("{}\u{001b}[48;5;239m{}\u{001b}[48;5;235m{}", left.collect::<String>(), current, right.collect::<String>());
                let _ = d.add_to_section(collected, &mut current_line_strategy, Alignment::Plus);
            } else {
                let collected = left.chain(right).collect::<String>();
                let _ = d.add_to_section(collected, &mut main_strategy, Alignment::Plus);
            }
        }
        // creates and prints the headers
        self.print_headers(headers, start);
        // prints the main drawprocess out to the terminal. 
        d.print(&mut CrosstermHandler, &mut stdout()).expect("Error queueing display instructions");
        // flushes the queued instructions out onto the screen. 
        CrosstermHandler::finish(&mut stdout()).expect("Error flushing display queue");
        // clears displays of text 
        d.clear(DividerStrategy::Beginning);
        headers.clear(DividerStrategy::Beginning);
    }
    pub fn print_headers(&mut self, headers: &mut DrawProcess, start: usize) {
        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut header_strategy = AnsiTruncate::new(ansi::GREEN, ansi::RESET);
        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut eof_strategy = AnsiTruncate::new(ansi::CYAN, ansi::RESET);
        // the line count of the program. 
        let total_length = self.contents.left.len() + self.contents.right.len();
        // for all lines in the file and on the display...
        for i in start..start + (headers.height().min(total_length)) {
            // print the number with the header_strategy format scheme
            let _ = headers.add_to_section(format!("{:-4} ", i), &mut header_strategy, Alignment::Plus);
        }
        // for all lines of exposed space in the display...
        for _ in total_length..headers.height() {
            // prints the character ~ with the eof_strategy format scheme
            let _ = headers.add_to_section(format!("   ~ "), &mut eof_strategy, Alignment::Plus);
        }
        // prints the header drawprocess out to the terminal. 
        headers.print(&mut CrosstermHandler, &mut stdout()).expect("Error queueing display instructions");
    }
}
#[derive(Debug)]
pub struct AnsiTruncate {
    pub left: &'static str,
    pub right: &'static str,
    pub extra_length: usize,
}
impl AnsiTruncate {
    pub fn new(left: &'static str, right: &'static str) -> AnsiTruncate {
        AnsiTruncate {
            left,
            right,
            extra_length: 0,
        }
    }
    pub fn add_length(&mut self, length: usize) {
        self.extra_length += length;
    }
}
impl Display for AnsiTruncate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl TrimStrategy for AnsiTruncate {
    type Input = String;
    fn trim(&mut self, text: String, chunk: &DrawProcess, _: Alignment) -> Vec<TrimmedText> {
        let blank_space = " ".graphemes(true).cycle();
        let orig = text.graphemes(true).chain(blank_space).take(chunk.width() + self.extra_length).collect::<String>();
        let res = format!("{}{}{}", self.left.to_string(), orig, self.right.to_string());
        self.extra_length = 0; // resets extra length. 
        vec![TrimmedText(res)]
    }
    fn back(&mut self, text: Vec<TrimmedText>, _: &DrawProcess, _: Alignment) -> Self::Input {
        text.into_iter().next().expect("Safe unwrap").0
    }
}