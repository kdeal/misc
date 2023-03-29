use std::io::{self, Write};

use crossterm::{
    self, cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};

#[derive(Debug, PartialEq, Eq)]
enum PromptMode {
    Normal,
    Insert,
}

struct PromptState {
    cursor: usize,
    line: String,
    mode: PromptMode,
}

impl PromptState {
    fn new() -> Self {
        PromptState {
            cursor: 0,
            line: String::new(),
            mode: PromptMode::Insert,
        }
    }

    fn max_cursor(&self) -> usize {
        match self.mode {
            PromptMode::Insert => self.line.len(),
            // -1 so you can only go to the last character and not past
            PromptMode::Normal => self.line.len() - 1,
        }
    }

    fn move_to_start(&mut self) {
        self.cursor = 0
    }

    fn move_to_end(&mut self) {
        self.cursor = self.max_cursor()
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1
        }
    }

    fn move_right(&mut self) {
        let max_cursor = self.max_cursor();
        if self.cursor < max_cursor {
            self.cursor += 1
        }
    }
}

fn handle_key(state: &mut PromptState, key: KeyCode) -> bool {
    match key {
        KeyCode::Enter => {
            return true;
        }
        KeyCode::Esc => {
            if state.mode != PromptMode::Normal {
                state.mode = PromptMode::Normal;
                state.move_left();
            }
        }
        KeyCode::Backspace => match state.mode {
            PromptMode::Normal => state.move_left(),
            PromptMode::Insert => {
                if state.cursor < state.max_cursor() {
                    if state.cursor != 0 {
                        state.line.remove(state.cursor - 1);
                        state.move_left();
                    }
                } else if state.line.pop().is_some() {
                    state.move_left();
                }
            }
        },
        KeyCode::Char(c) => match state.mode {
            PromptMode::Normal => handle_normal_mode_key(state, c),
            PromptMode::Insert => {
                if state.cursor < state.max_cursor() {
                    state.line.insert(state.cursor, c);
                } else {
                    state.line.push(c);
                }
                state.move_right();
            }
        },
        _ => {}
    }
    false
}

fn handle_normal_mode_key(state: &mut PromptState, c: char) {
    match c {
        'i' => state.mode = PromptMode::Insert,
        'I' => {
            state.mode = PromptMode::Insert;
            state.move_to_start();
        }
        'a' => {
            state.mode = PromptMode::Insert;
            state.move_right();
        }
        'A' => {
            state.mode = PromptMode::Insert;
            state.move_to_end();
        }
        'h' => state.move_left(),
        'l' => state.move_right(),
        _ => {}
    }
}

pub fn basic_prompt(prompt: &str) -> anyhow::Result<String> {
    let mut stderr = io::stderr();
    eprint!("{} ", prompt);
    stderr.flush()?;

    let mut state = PromptState::new();
    let input_start = u16::try_from(prompt.len() + 1)?;

    enable_raw_mode()?;
    stderr.execute(cursor::SetCursorStyle::SteadyBar)?;

    while let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event::read()?
    {
        if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
            break;
        }
        if handle_key(&mut state, code) {
            break;
        }

        let (_, position_row) = cursor::position()?;
        let cursor_shape = match state.mode {
            PromptMode::Normal => cursor::SetCursorStyle::SteadyBlock,
            PromptMode::Insert => cursor::SetCursorStyle::SteadyBar,
        };
        stderr
            .queue(cursor::MoveTo(input_start, position_row))?
            .queue(Clear(ClearType::UntilNewLine))?
            .queue(Print(&state.line))?
            .queue(cursor::MoveTo(
                input_start + u16::try_from(state.cursor)?,
                position_row,
            ))?
            .queue(cursor_shape)?
            .flush()?;
    }
    disable_raw_mode()?;
    eprintln!();

    Ok(state.line)
}
