use std::io::{self, Write};

use anyhow::bail;
use crossterm::{
    self, cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{self, Color},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, ScrollUp},
    ExecutableCommand, QueueableCommand,
};

#[derive(Debug, PartialEq, Eq)]
enum PromptMode {
    Normal,
    Insert,
}

struct PromptState {
    cursor: usize,
    input_start: u16,
    input_row: u16,
    line: String,
    mode: PromptMode,
}

impl PromptState {
    fn new(input_start: u16, input_row: u16) -> Self {
        PromptState {
            cursor: 0,
            input_start,
            input_row,
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

fn determine_cursor_shape(state: &PromptState) -> cursor::SetCursorStyle {
    match state.mode {
        PromptMode::Normal => cursor::SetCursorStyle::SteadyBlock,
        PromptMode::Insert => cursor::SetCursorStyle::SteadyBar,
    }
}

fn handle_key(
    state: &mut PromptState,
    key: KeyCode,
    modifiers: KeyModifiers,
) -> anyhow::Result<bool> {
    match (&state.mode, key, modifiers) {
        (_, KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            bail!("ctrl-c sent");
        }
        (mode, keycode, KeyModifiers::NONE) => match (mode, keycode) {
            (_, KeyCode::Enter) => {
                return Ok(true);
            }
            (PromptMode::Insert, KeyCode::Esc) => {
                state.mode = PromptMode::Normal;
                state.move_left();
            }
            (PromptMode::Normal, KeyCode::Backspace) => state.move_left(),
            (PromptMode::Insert, KeyCode::Backspace) => {
                if state.cursor < state.max_cursor() {
                    if state.cursor != 0 {
                        state.line.remove(state.cursor - 1);
                        state.move_left();
                    }
                } else if state.line.pop().is_some() {
                    state.move_left();
                }
            }
            (PromptMode::Insert, KeyCode::Char(c)) => {
                if state.cursor < state.max_cursor() {
                    state.line.insert(state.cursor, c);
                } else {
                    state.line.push(c);
                }
                state.move_right();
            }
            (PromptMode::Normal, KeyCode::Char(c)) => match c {
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
            },
            (_, _) => {}
        },
        (_, _, _) => {}
    }
    Ok(false)
}

fn print_prompt_input(state: &PromptState, stderr: &mut dyn Write) -> anyhow::Result<()> {
    stderr
        .queue(cursor::MoveTo(state.input_start, state.input_row))?
        .queue(Clear(ClearType::UntilNewLine))?
        .queue(style::Print(&state.line))?;
    Ok(())
}

fn update_cursor(state: &PromptState, stderr: &mut dyn Write) -> anyhow::Result<()> {
    stderr
        .queue(cursor::MoveTo(
            state.input_start + u16::try_from(state.cursor)?,
            state.input_row,
        ))?
        .queue(determine_cursor_shape(state))?;
    Ok(())
}

pub fn basic_prompt(prompt: &str) -> anyhow::Result<String> {
    let mut stderr = io::stderr();
    eprint!("{} ", prompt);
    stderr.flush()?;

    let input_start = u16::try_from(prompt.len() + 1)?;
    let (_, input_row) = cursor::position()?;
    let mut state = PromptState::new(input_start, input_row);

    enable_raw_mode()?;
    stderr.execute(cursor::SetCursorStyle::SteadyBar)?;

    while let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event::read()?
    {
        if handle_key(&mut state, code, modifiers)? {
            break;
        }

        print_prompt_input(&state, &mut stderr)?;
        update_cursor(&state, &mut stderr)?;
        stderr.flush()?;
    }
    disable_raw_mode()?;
    eprintln!();

    Ok(state.line)
}

struct SelectionState {
    selected: u16,
    num_items: u16,
    max_index: u16,
    prompt_state: PromptState,
}

impl SelectionState {
    fn new(num_items: u16, input_start: u16, input_row: u16) -> Self {
        SelectionState {
            selected: 0,
            num_items,
            max_index: num_items - 1,
            prompt_state: PromptState::new(input_start, input_row),
        }
    }

    fn update_max_index(&mut self, max_index: u16) {
        self.max_index = max_index;
        if self.selected > self.max_index {
            self.selected = self.max_index
        }
    }

    fn next_item(&mut self) {
        if self.selected < self.max_index {
            self.selected += 1
        }
    }

    fn previous_item(&mut self) {
        if self.selected > 0 {
            self.selected -= 1
        }
    }
}

fn select_handle_key(
    state: &mut SelectionState,
    key: KeyCode,
    modifiers: KeyModifiers,
) -> anyhow::Result<bool> {
    match (&state.prompt_state.mode, key, modifiers) {
        (PromptMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE) => state.next_item(),
        (PromptMode::Normal, KeyCode::Char('k'), KeyModifiers::NONE) => state.previous_item(),
        (_, _, _) => return handle_key(&mut state.prompt_state, key, modifiers),
    };
    Ok(false)
}

fn print_options(
    state: &SelectionState,
    #[allow(clippy::ptr_arg)] options: &Vec<&String>,
    stderr: &mut dyn Write,
) -> anyhow::Result<()> {
    stderr.queue(Clear(ClearType::FromCursorDown))?;
    let selected_usize = usize::from(state.selected);
    for (i, option) in options.iter().enumerate() {
        if i == selected_usize {
            stderr
                .queue(style::SetForegroundColor(Color::DarkCyan))?
                .queue(style::Print("> "))?
                .queue(style::SetAttribute(style::Attribute::Bold))?;
        } else {
            stderr
                .queue(style::Print("  "))?
                .queue(style::SetForegroundColor(Color::Reset))?
                .queue(style::SetAttribute(style::Attribute::Reset))?;
        }
        stderr.queue(style::Print(&option))?;
        if i < 10 {
            stderr.queue(cursor::MoveToNextLine(1))?;
        }
    }
    stderr
        .queue(style::SetForegroundColor(Color::Reset))?
        .queue(style::SetAttribute(style::Attribute::Reset))?;
    Ok(())
}

fn filter_options<'a>(filter: &str, options: &'a [String]) -> Vec<&'a String> {
    options
        .iter()
        .filter(|option| option.contains(filter))
        .collect()
}

pub fn select_prompt<'a>(prompt: &str, options: &'a Vec<String>) -> anyhow::Result<&'a str> {
    let mut stderr = io::stderr();
    eprint!("{} ", prompt);
    stderr.flush()?;

    let num_items = 10.min(options.len());
    let input_start = u16::try_from(prompt.len() + 1)?;
    let mut state = SelectionState::new(u16::try_from(num_items)?, input_start, 0);

    enable_raw_mode()?;
    stderr
        .queue(ScrollUp(state.num_items))?
        .queue(cursor::MoveToPreviousLine(state.num_items - 1))?;
    let (_, position_row) = cursor::position()?;
    print_options(&state, &options.iter().collect(), &mut stderr)?;
    // We shifted the input row, so we need to update it
    state.prompt_state.input_row = position_row - 1;
    update_cursor(&state.prompt_state, &mut stderr)?;
    stderr.flush()?;

    while let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event::read()?
    {
        if select_handle_key(&mut state, code, modifiers)? {
            break;
        }

        let filtered_options = filter_options(&state.prompt_state.line, options);
        let new_num_items = state
            .num_items
            .min(u16::try_from(filtered_options.len()).unwrap_or(state.num_items));
        state.update_max_index(if new_num_items > 0 {
            new_num_items - 1
        } else {
            0
        });

        print_prompt_input(&state.prompt_state, &mut stderr)?;
        stderr.queue(cursor::MoveToNextLine(1))?;
        print_options(&state, &filtered_options, &mut stderr)?;
        update_cursor(&state.prompt_state, &mut stderr)?;
        stderr.flush()?;
    }
    stderr
        .queue(cursor::MoveToNextLine(1))?
        .queue(Clear(ClearType::FromCursorDown))?
        .flush()?;
    disable_raw_mode()?;

    Ok(&options[usize::from(state.selected)])
}
