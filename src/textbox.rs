use std::io::stdout;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use grid_ui::{
    crossterm::CrosstermHandler,
    grid::{Alignment, DividerStrategy},
    process::DrawProcess,
};

use crate::{ansi, ansi_truncate, State, debug};

pub const UNDOS: usize = 10;

pub fn kind(c: char) -> i8 {
    if c.is_ascii_alphabetic() {           // alpha
        0b_0000_0001
    } else if c.is_ascii_alphanumeric() {  // numeric
        0b_0000_0010
    }else if c.is_ascii_punctuation() {    // punctuation
        0b_0000_0100
    } else if c.is_ascii_whitespace() {    // whitespace
        0b_0000_1111
    } else {                               // other
        0b_0000_1000
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub x_pos: usize,
    pub y_pos: usize,
    pub highlight: Option<(usize, usize)>,
}
impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            x_pos: 0,
            y_pos: 0,
            highlight: None,
        }
    }
}
pub struct TextBoxUndos {
    pub cursor: Cursor,
    pub contents: Vec<Vec<char>>,
}
pub struct TextBoxHistory {
    pos: usize,
    undos: Vec<TextBoxUndos>,
}
impl TextBoxHistory {
    pub fn new() -> TextBoxHistory {
        TextBoxHistory { pos: 0, undos: Vec::new() }
    }
    pub fn start(&mut self) {
        self.pos = 0;
    }
    pub fn action(&mut self, tbco: &mut Vec<Vec<char>>, tbcu: &mut Cursor) {
        self.pos += 1;
        if self.pos > UNDOS {
            self.undos.remove(0);
            self.pos -= 1;
        }
        self.undos.push(TextBoxUndos {
            cursor: tbcu.clone(),
            contents: tbco.clone(),
        });
        if self.pos < self.undos.len() - 1 {
            self.undos.drain((self.pos + 1)..);
        }
        
    }
    pub fn undo(&mut self, tbco: &mut Vec<Vec<char>>, tbcu: &mut Cursor) {
        debug::debug(&format!("UNDOING! POS: {}", self.pos));
        if self.pos > 0 {
            debug::debug(&format!("WE DID IT!!!"));
            self.pos -= 1;
            let undo = &self.undos[self.pos];
            *tbcu = undo.cursor.clone();
            *tbco = undo.contents.clone();
        }
    }
    pub fn redo(&mut self, tbco: &mut Vec<Vec<char>>, tbcu: &mut Cursor) {
        if self.pos < self.undos.len() - 1 {
            self.pos += 1;
            let redo = &self.undos[self.pos];
            *tbcu = redo.cursor.clone();
            *tbco = redo.contents.clone();
        }
    }
}
pub struct TextBox {
    cursor: Cursor,
    contents: Vec<Vec<char>>,
    path: String,
    history: TextBoxHistory,
}
impl TextBox {
    pub fn new(lines: Vec<String>, path: String) -> TextBox {
        let mut tb = TextBox {
            cursor: Cursor::new(),
            contents: lines.into_iter().map(|x| x.chars().collect()).collect(),
            path,
            history: TextBoxHistory::new(),
        };
        tb.history.action(&mut tb.contents, &mut tb.cursor);
        tb.history.start();
        tb
    }
    pub fn action(&mut self) {
        self.history.action(&mut self.contents, &mut self.cursor);
    }
    pub fn set_valid_pos(&mut self) {
        let len = self.contents[self.cursor.y_pos].len();
        if self.cursor.x_pos > len {
            self.cursor.x_pos = len;
        }
    }
    pub fn set_valid_pos_h(&mut self) {
        if let Some((x, y)) = self.cursor.highlight {
            let len = self.contents[y].len();
            if x > len {
                self.cursor.highlight = Some((len, y));
            }
        }
    }
    pub fn highlight(&mut self) -> Option<String> {
        self.set_valid_pos();
        self.set_valid_pos_h();
        let (x2, y2) = self.cursor.highlight?;
        let (x1, y1) = (self.cursor.x_pos, self.cursor.y_pos);
        if y1 == y2 {
            // if the highlight is on one line... return the characters between the two x positions.
            let min = x1.min(x2);
            let max = x1.max(x2);
            Some(self.contents[y1][min..max].iter().collect::<String>())
        } else if y1 < y2 {
            // if the highlight goes downwards...
            let mut s = String::new();
            // return the characters after the cursor position...
            s.push_str(&self.contents[y1][x1..].iter().collect::<String>());
            for i in y1 + 1..y2 {
                // all the lines in between...
                s.push_str(&self.contents[i].iter().collect::<String>());
            }
            // and before the highlight position. 
            s.push_str(&self.contents[y1][..x2].iter().collect::<String>());
            Some(s)
        } else { // y2 < y1
            // if the highlight goes upwards...
            let mut s = String::new();
            // return the characters after the highlight position...
            s.push_str(&self.contents[y2][x2..].iter().collect::<String>());
            for i in y2 + 1..y1 {
                // all the lines in between...
                s.push_str(&self.contents[i].iter().collect::<String>());
            }
            // and before the cursor position.
            s.push_str(&self.contents[y1][..x1].iter().collect::<String>());
            Some(s)
        }
    }
    pub fn remove_highlight(&mut self) {
        self.set_valid_pos();
        self.set_valid_pos_h();
        if let Some((x2, y2)) = self.cursor.highlight {
            let (x1, y1) = (self.cursor.x_pos, self.cursor.y_pos);
            if y1 == y2 {
                // if the highlight is on one line... remove the characters between the two x positions.
                let min = x1.min(x2);
                let max = x1.max(x2);
                self.contents[y1].drain(min..max);
                self.cursor.x_pos = min;
            } else if y1 < y2 {
                self.cursor.x_pos = x1;
                self.cursor.y_pos = y1;
                // if the highlight goes downwards... remove the characters after the cursor position
                self.contents[y1].drain(x1..);
                // and before the highlight position. 
                self.contents[y2].drain(..x2);
                let mut removed = self.contents.remove(y2);
                // Append them onto the previous line
                self.contents[y1].append(&mut removed);
                for _ in y1 + 1..y2 {
                    // and remove all the lines in between.
                    self.contents.remove(y1 + 1);
                }
            } else { // y2 < y1
                self.cursor.x_pos = x2;
                self.cursor.y_pos = y2;
                // if the highlight goes upwards... remove the characters after the highlight position
                self.contents[y2].drain(x2..);
                // and before the cursor position. 
                self.contents[y1].drain(..x1);
                let mut removed = self.contents.remove(y1);
                // Append them onto the first line
                self.contents[y2].append(&mut removed);
                for _ in y2 + 1..y1 {
                    // and remove all the lines in between. 
                    self.contents.remove(y2 + 1);
                }
            }
            if y1 != y2 || x1 != x2 {
                self.action();
            }
        }
        self.cursor.highlight = None;
        self.set_valid_pos();
    }
    // Calculates the length of the word at the cursor.
    // words are defined as a sequence of numbers, letters, punctuation/symbols, or whitespace characters. 
    pub fn word_length(&self, x: usize, y: usize, reverse: bool) -> usize {
        if !reverse {
            // goes forward
            if x == self.contents[y].len() {
                return 1;
            }
            let mut offset = 1;
            let line = &self.contents[y];
            let mut kind_res = kind(line[x]);
            while offset + x < line.len() 
                && kind(line[offset + x]) >= 0 {
                    kind_res &= kind(line[offset + x]);
                    if kind_res != kind(line[offset + x]) {
                        break;
                    }
                    offset += 1;
            }
            offset
        } else {
            // goes backward
            if x == 0 || x == 1 {
                return 1;
            }
            let mut offset = 1;
            let line = &self.contents[y];
            let mut kind_res = kind(line[x - 1]);
            while x - offset > 0 
                && kind(line[x - offset - 1]) >= 0 {
                    kind_res &= kind(line[x - offset - 1]);
                    if kind_res != kind(line[x - offset - 1]){
                        break;
                    }
                    offset += 1;
            }
            debug::debug(&format!("GOT REV {}", offset));
            offset
        }
    }
    pub fn word_length_toggle(&self, reverse: bool, x: usize, y: usize, ctrl: bool) -> usize {
        if ctrl { self.word_length(x, y, reverse) } else { 1 }
    }
    pub fn word_length_toggle_c(&self, reverse: bool, ctrl: bool) -> usize {
        self.word_length_toggle(reverse, self.cursor.x_pos, self.cursor.y_pos, ctrl)
    }
    pub fn word_length_toggle_h(&self, reverse: bool, ctrl: bool) -> usize {
        let (x, y) = self.cursor.highlight.unwrap_or((self.cursor.x_pos, self.cursor.y_pos));
        self.word_length_toggle(reverse, x, y, ctrl)
    }
    // Handles the key press. 
    pub fn recv_key(
        &mut self,
        k: KeyEvent,
        d: &mut DrawProcess,
        headers: &mut DrawProcess,
        state: &mut State,
    ) {
        let KeyEvent { code, modifiers } = k;
        match code {
            // deletes the previous character if there is one. Merges two lines if needed.
            KeyCode::Backspace => {
                self.set_valid_pos();
                if let Some(_) = self.cursor.highlight {
                    self.remove_highlight();
                } else {
                    for _ in 0..self.word_length_toggle_c(true, modifiers.contains(KeyModifiers::CONTROL)) {
                        // if we're at the beginning of a line...
                        if self.cursor.x_pos == 0 {
                            // if we're not on the first line...
                            if self.cursor.y_pos != 0 {
                                // move the cursor to the end of the previous line
                                self.cursor.x_pos = self.contents[self.cursor.y_pos - 1].len();
                                // add the current line onto the previous line
                                let mut this_line = self.contents.remove(self.cursor.y_pos);
                                self.contents[self.cursor.y_pos - 1].append(&mut this_line);
                                // we're on the previous line now
                                self.cursor.y_pos -= 1;
                            }
                        } else {
                            self.contents[self.cursor.y_pos].remove(self.cursor.x_pos - 1);
                            self.cursor.x_pos -= 1;
                        }
                    }
                    self.action();
                }
            }
            // Deletes the next character if there is one. Merges two lines if needed.
            KeyCode::Delete => {
                self.set_valid_pos();
                if let Some(_) = self.cursor.highlight {
                    self.remove_highlight();
                } else {
                    for _ in 0..self.word_length_toggle_c(false, modifiers.contains(KeyModifiers::CONTROL)) {
                        // if we're at the end of a line...
                        if self.cursor.x_pos == self.contents[self.cursor.y_pos].len() {
                            // if we're not on the last line...
                            if self.cursor.y_pos != self.contents.len() - 1 {
                                // add the current line onto the next line
                                let mut next_line = self.contents.remove(self.cursor.y_pos + 1);
                                self.contents[self.cursor.y_pos].append(&mut next_line);
                            }
                        } else {
                            self.contents[self.cursor.y_pos].remove(self.cursor.x_pos);
                        }
                    }
                    self.action();
                }
            }
            // splits the current line into two lines.
            KeyCode::Enter => {
                self.set_valid_pos();
                self.remove_highlight();
                let new_line = self.contents[self.cursor.y_pos].split_off(self.cursor.x_pos);
                self.contents.insert(self.cursor.y_pos + 1, new_line);
                self.cursor.x_pos = 0;
                self.cursor.y_pos += 1;
                self.action();
            }
            // moves the cursor leftwards, or to the end of the previous line.
            KeyCode::Left => {
                self.set_valid_pos();
                self.set_valid_pos_h();
                if modifiers.contains(KeyModifiers::SHIFT) {
                    let amt = if let Some(_) = self.cursor.highlight {
                        self.word_length_toggle_h(true, modifiers.contains(KeyModifiers::CONTROL))
                    } else {
                        self.word_length_toggle_c(true, modifiers.contains(KeyModifiers::CONTROL))
                    };
                    for _ in 0..amt {
                        if let Some((x, y)) = self.cursor.highlight {
                            if x > 0 {
                                self.cursor.highlight = Some((x - 1, y));
                            } else if y > 0 {
                                self.cursor.highlight = Some((self.contents[y - 1].len(), y - 1));
                            }
                        } else {
                            if self.cursor.x_pos > 0 {
                                self.cursor.highlight = Some((self.cursor.x_pos - 1, self.cursor.y_pos));
                            } else if self.cursor.y_pos > 0 {
                                self.cursor.highlight = Some((self.contents[self.cursor.y_pos - 1].len(), self.cursor.y_pos - 1));
                            }
                        }
                    }
                } else {
                    for _ in 0..self.word_length_toggle_c(true, modifiers.contains(KeyModifiers::CONTROL)) {
                        self.cursor.highlight = None;
                        if self.cursor.x_pos > 0 {
                            self.cursor.x_pos -= 1;
                        } else if self.cursor.y_pos > 0 {
                            self.cursor.y_pos -= 1;
                            self.cursor.x_pos = self.contents[self.cursor.y_pos].len();
                        }
                    }
                }
            }
            // moves the cursor rightwards, or to the beginning of the next line.
            KeyCode::Right => {
                self.set_valid_pos();
                self.set_valid_pos_h();
                if modifiers.contains(KeyModifiers::SHIFT) {
                    let amt = if let Some(_) = self.cursor.highlight {
                        self.word_length_toggle_h(false, modifiers.contains(KeyModifiers::CONTROL))
                    } else {
                        self.word_length_toggle_c(false, modifiers.contains(KeyModifiers::CONTROL))
                    };
                    for _ in 0..amt {
                        if let Some((x, y)) = self.cursor.highlight {
                            if x < self.contents[y].len() {
                                self.cursor.highlight = Some((x + 1, y));
                            } else if y < self.contents.len() - 1 {
                                self.cursor.highlight = Some((0, y + 1));
                            }
                        } else {
                            if self.cursor.x_pos < self.contents[self.cursor.y_pos].len() {
                                self.cursor.highlight = Some((self.cursor.x_pos + 1, self.cursor.y_pos));
                            } else if self.cursor.y_pos < self.contents.len() - 1 {
                                self.cursor.highlight = Some((0, self.cursor.y_pos + 1));
                            }
                        }
                    }
                } else {
                    for _ in 0..self.word_length_toggle_c(false, modifiers.contains(KeyModifiers::CONTROL)) {
                        self.cursor.highlight = None;
                        if self.cursor.x_pos < self.contents[self.cursor.y_pos].len() {
                            self.cursor.x_pos += 1;
                        } else if self.cursor.y_pos < self.contents.len() - 1 {
                            self.cursor.y_pos += 1;
                            self.cursor.x_pos = 0;
                        }
                    }
                }
            }
            // moves the cursor upwards, or to the end of the last line (if on the last line)
            KeyCode::Up => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    if let Some((x, y)) = self.cursor.highlight {
                        if y > 0 {
                            self.cursor.highlight = Some((x, y - 1));
                        } else {
                            self.cursor.highlight = Some((0, 0));
                        }
                    } else {
                        if self.cursor.y_pos > 0 {
                            self.cursor.highlight = Some((self.cursor.x_pos, self.cursor.y_pos - 1));
                        } else {
                            self.cursor.highlight = Some((0, 0));
                        }
                    }
                } else {
                    self.cursor.highlight = None;
                    if self.cursor.y_pos > 0 {
                        self.cursor.y_pos -= 1;
                    } else {
                        self.cursor.x_pos = 0;
                    }
                }
            }
            // moves the cursor downwards, or to the beginning of the first line (if on the first line).
            KeyCode::Down => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    if let Some((x, y)) = self.cursor.highlight {
                        if y < self.contents.len() - 1 {
                            self.cursor.highlight = Some((x, y + 1));
                        } else {
                            self.cursor.highlight = Some((self.contents[y].len(), y));
                        }
                    } else {
                        if self.cursor.y_pos < self.contents.len() - 1 {
                            self.cursor.highlight = Some((self.cursor.x_pos, self.cursor.y_pos + 1));
                        } else {
                            self.cursor.highlight = Some((self.contents[self.cursor.y_pos].len(), self.cursor.y_pos));
                        }
                    }
                } else {
                    self.cursor.highlight = None;
                    if self.cursor.y_pos < self.contents.len() - 1 {
                        self.cursor.y_pos += 1;
                    } else {
                        self.cursor.x_pos = self.contents.get(self.cursor.y_pos).unwrap().len();
                    }
                }
            }
            // Inserts a tab character.
            KeyCode::Tab => {
                self.set_valid_pos();
                self.remove_highlight();
                self.contents[self.cursor.y_pos].insert(self.cursor.x_pos, '\t');
                self.cursor.x_pos += 1;
                self.action();
            }
            // Either executes a control sequence or adds a key.
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.ctrl_keys(c, modifiers, state);
                } else {
                    self.set_valid_pos();
                    self.remove_highlight();
                    self.contents[self.cursor.y_pos].insert(self.cursor.x_pos, c);
                    self.cursor.x_pos += 1;
                    self.action();
                }
            }
            // Escape currently does nothing, but might do something in the future. 
            KeyCode::Esc => {}
            // No other key presses currently do anything.
            _ => {}
        }
        self.display(d, headers);
    }
    // Executes a control sequence, and returns true if the program should end.
    pub fn ctrl_keys(&mut self, c: char, m: KeyModifiers, state: &mut State) {
        match c {
            // ctrl+c will copy text in the future
            'c' | 'C' => {
                if let Some(val) = self.highlight() {
                    let _ = cli_clipboard::set_contents(val);
                }
            }
            // If ctrl+q is pressed, the program should end.
            'q' | 'Q' => {
                state.will_quit = true;
            }
            // ctrl+v will paste text in the future
            'v' | 'V' => {
                if let Ok(str) = cli_clipboard::get_contents() {
                    self.remove_highlight();
                    for c in str.chars() {
                        self.contents[self.cursor.y_pos].insert(self.cursor.x_pos, c);
                        self.cursor.x_pos += 1;
                    }
                    self.action();
                }
            }
            'z' | 'Z' => {
                if m.contains(KeyModifiers::SHIFT) {
                    debug::debug("CONTROL SHIFT ZEEEEE\n");
                    self.history.redo(&mut self.contents, &mut self.cursor);
                } else {
                    debug::debug("CONTROL ZEEEEE\n");
                    self.history.undo(&mut self.contents, &mut self.cursor);
                }
            }
            'y' | 'Y' => {
                if m.contains(KeyModifiers::SHIFT) {
                    debug::debug("CONTROL SHIFT WHYYY\n");
                    self.history.redo(&mut self.contents, &mut self.cursor);
                } else {
                    debug::debug("CONTROL WHY\n");
                    self.history.redo(&mut self.contents, &mut self.cursor);
                }
            }
            // ctrl+x will cut text in the future
            'x' | 'X' => {
                self.ctrl_keys('c', m, state);
                self.remove_highlight();
            }
            // ctrl+s saves the file
            's' | 'S' => {
                if let Err(_) = std::fs::write(
                    &self.path,
                    self.contents
                        .iter() // iterates through the lines
                        .map(|x| x.iter().collect::<String>()) // collects each line into a string
                        .collect::<Vec<String>>() // collects each line into a vector of lines
                        .join("\n"),
                ) {
                    // joins the lines with a newline character
                    // attempts to write the resulting data. If the write fails, prints a message.
                    println!("Failed to save!");
                }
            }
            _ => {}
        }
    }
    // Calculates the position where the display starts printing.
    pub fn calculate_start(&mut self, height: usize) -> usize {
        let current_line = self.cursor.y_pos;
        let half_pos = height / 2;
        let total_length = self.contents.len();
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
        let mut min_x;
        let min_y;
        let mut max_x;
        let max_y;
        let highlight; 
        if let Some((x1, y1)) = self.cursor.highlight {
            highlight = ansi::BACKGROUND_CYAN;
            let (x2, y2) = (self.cursor.x_pos, self.cursor.y_pos);
            match y1.cmp(&y2) {
                std::cmp::Ordering::Less => {
                    min_x = x1;
                    min_y = y1;
                    max_x = x2;
                    max_y = y2;
                },
                std::cmp::Ordering::Equal => {
                    min_x = x1.min(x2);
                    min_y = y1;
                    max_x = x1.max(x2);
                    max_y = y1;
                },
                std::cmp::Ordering::Greater => {
                    min_x = x2;
                    min_y = y2;
                    max_x = x1;
                    max_y = y1;
                },
            }
        } else {
            highlight = ansi::SELECTED;
            min_x = self.cursor.x_pos;
            min_y = self.cursor.y_pos;
            max_x = self.cursor.x_pos;
            max_y = self.cursor.y_pos;
        }
        // at least one character is highlighted, so max_x must be at least min_x + 1.
        if max_x == min_x && min_y == max_y {
            max_x += 1;
        }
        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut main_strategy = ansi_truncate::AnsiTruncate::new(ansi::RESET, ansi::RESET);
        let mut current_line_strategy =
            ansi_truncate::AnsiTruncate::new(ansi::SELECTED_LINE, ansi::RESET);
        // enumerates through the contents.
        for (mut i, line) in self.contents[start..(start + d.height()).min(self.contents.len())].iter().enumerate() {
            i += start;
            // if we're on the only highlighted line... 
            if i == min_y && i == max_y {
                // the current string to be printed
                max_x = max_x.min(line.len());
                min_x = min_x.min(line.len());
                
                let mut current = if line.len() == 0 {" ".to_string()} else {line[min_x..max_x].iter().collect::<String>()};
                if current.is_empty() {
                    current = " ".to_string();
                }
                // formats the string with the current character highlighted - the left portion, formatting for the current character,
                // the current character, default formatting, and then the right portion.
                let collected = if max_x == line.len() {
                    format!(
                        "{}{}{}\u{001b}[48;5;235m{}",
                        line[..min_x].iter().collect::<String>(),
                        highlight,
                        current,
                        ""
                    )
                } else {
                    format!(
                        "{}{}{}\u{001b}[48;5;235m{}",
                        line[..min_x].iter().collect::<String>(),
                        highlight,
                        current,
                        line[max_x..].iter().collect::<String>()
                    )
                };
                // adds it to the section
                let _ = d.add_to_section(collected, &mut current_line_strategy, Alignment::Plus);
            } else if i == min_y {
                let collected = if min_x >= line.len() {
                    format!("{}{}{}", 
                        line.iter().collect::<String>(), 
                        highlight, 
                        " "
                    )
                } else {
                    format!("{}{}{}", 
                        line[..min_x].iter().collect::<String>(), 
                        highlight, 
                        line[min_x..].iter().collect::<String>()
                    )
                };
                // adds it to the selection
                let _ = d.add_to_section(collected, &mut main_strategy, Alignment::Plus);
            } else if i > min_y && i < max_y {
                let collected = format!("{}{}", highlight, line.iter().collect::<String>());
                let _ = d.add_to_section(collected, &mut main_strategy, Alignment::Plus);

            } else if i == max_y {
                let collected = if max_x >= line.len() {
                    format!("{}{} ", 
                        highlight, 
                        line.iter().collect::<String>(),
                    )
                } else {
                    format!("{}{}{}{}", 
                        highlight,
                        line[..max_x].iter().collect::<String>(),
                        ansi::RESET,
                        line[max_x..].iter().collect::<String>()
                    )
                };
                let _ = d.add_to_section(collected, &mut main_strategy, Alignment::Plus);
            } else {
                // otherwise, just prints the whole line
                let collected = line.iter().collect::<String>();
                let _ = d.add_to_section(collected, &mut main_strategy, Alignment::Plus);
            }
        }
        // creates and prints the headers
        self.print_headers(headers, start);

        // prints the main drawprocess out to the terminal.
        d.print(&mut CrosstermHandler, &mut stdout())
            .expect("Error queueing display instructions");

        // flushes the queued instructions out onto the screen.
        CrosstermHandler::finish(&mut stdout()).expect("Error flushing display queue");

        // clears displays of text
        d.clear(DividerStrategy::Beginning);
        headers.clear(DividerStrategy::Beginning);
    }
    pub fn print_headers(&mut self, headers: &mut DrawProcess, start: usize) {
        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut header_strategy = ansi_truncate::AnsiTruncate::new(ansi::GREEN, ansi::RESET);

        // removes all extra spaces and inserts these formatting codes at the beginning/end:
        let mut eof_strategy = ansi_truncate::AnsiTruncate::new(ansi::CYAN, ansi::RESET);

        // the line count of the program.
        let total_length = self.contents.len();

        // for all lines in the file and on the display...
        for i in start..start + (headers.height().min(total_length)) {
            // print the number with the header_strategy format scheme
            let _ =
                headers.add_to_section(format!("{:-4} ", i), &mut header_strategy, Alignment::Plus);
        }
        // for all lines of exposed space in the display...
        for _ in total_length..headers.height() {
            // prints the character ~ with the eof_strategy format scheme
            let _ = headers.add_to_section(format!("   ~ "), &mut eof_strategy, Alignment::Plus);
        }

        // prints the header drawprocess out to the terminal.
        headers
            .print(&mut CrosstermHandler, &mut stdout())
            .expect("Error queueing display instructions");
    }
}
