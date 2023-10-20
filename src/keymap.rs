use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KeyLevels {
    pub levels: Vec<KeyLevel>,
}
impl KeyLevels {
    pub fn map_keys(&self, event: KeyEvent, current_mode: Mode) -> Vec<KeyEvent> {
        let mut queue = Vec::new();
        queue.push(event);
        // iterates through each level
        for level in &self.levels {
            // a loop because of the recursion portion of the level
            loop {
                // if the flag is still equal to false, the loop will end.
                let mut flag = false;
                // moves the queue over to a new vector
                let new_queue = std::mem::replace(&mut queue, Vec::new());
                // iterates through the queue
                for item in new_queue {
                    let mut replaced = false;
                    // iterates through the possible replacements
                    for line in &level.recurse {
                        // if there is a replacement...
                        if line.mode_req.matches(current_mode) && line.pattern == item {
                            // adds the replacement
                            queue.append(&mut line.result.clone());
                            flag = true;
                            replaced = true;
                            break;
                        }
                    }
                    // If there hasn't been a replacement, add the item, unaltered.
                    if !replaced {
                        queue.push(item);
                    }
                }
                // if it's left unaltered, break.
                if !flag {
                    break;
                }
            }
            // moves the queue over to a new vector
            let new_queue = std::mem::replace(&mut queue, Vec::new());
            for item in new_queue {
                let mut replaced: bool = false;
                for line in &level.fall {
                    // if there is a replacement...
                    if line.mode_req.matches(current_mode) && line.pattern == item {
                        // adds the replacement
                        queue.append(&mut line.result.clone());
                        replaced = true;
                        break;
                    }
                }
                if !replaced {
                    // if there's no replacement, pushes the item on, unaltered.
                    queue.push(item);
                }
            }
        }
        return queue;
    }
}

#[derive(Serialize, Deserialize)]
pub struct KeyLevel {
    pub recurse: Vec<KeyMap>,
    pub fall: Vec<KeyMap>,
}
#[derive(Serialize, Deserialize)]
pub struct KeyMap {
    pub pattern: KeyEvent,
    pub result: Vec<KeyEvent>,
    pub mode_req: ModeReq,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ModeReq {
    Any = 0,
    Insert = 1,
    Command = 2,
}
impl ModeReq {
    pub fn matches(&self, mode: Mode) -> bool {
        (*self as u8) == 0 || (*self as u8) == (mode as u8)
    }
}
#[derive(Clone, Copy)]
pub enum Mode {
    Insert = 1,
    Command = 2,
}


