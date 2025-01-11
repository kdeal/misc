use std::io::{self, Write};

use anyhow::bail;
use crossterm::{
    self, cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{self, Attribute, Color, PrintStyledContent, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

const MAX_OPTIONS_SHOWN: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
enum OpAdjust {
    Empty,
    Around,
    Inner,
}

#[derive(Debug, PartialEq, Eq)]
enum Operation {
    Change(OpAdjust),
    Delete(OpAdjust),
}

#[derive(Debug, PartialEq, Eq)]
enum PromptMode {
    Normal,
    Insert,
    OperatorPending(Operation),
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
            PromptMode::Normal | PromptMode::OperatorPending(_) => self.line.len() - 1,
        }
    }

    fn insert_mode(&mut self) {
        self.mode = PromptMode::Insert
    }

    fn normal_mode(&mut self) {
        self.mode = PromptMode::Normal
    }

    fn operator_pending_mode(&mut self, op: Operation) {
        self.mode = PromptMode::OperatorPending(op)
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

    fn move_to_current_word_end(&mut self) {
        if self.cursor < self.max_cursor() {
            self.cursor = self.get_current_word_end();
        }
    }

    fn move_to_current_word_start(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.get_current_word_start();
        }
    }

    fn move_to_next_word_start(&mut self) {
        if self.cursor < self.max_cursor() {
            self.cursor = self.get_next_word_start();
        }
    }

    fn get_current_word_end(&self) -> usize {
        fn predicate(item: &(usize, char)) -> bool {
            !item.1.is_alphanumeric()
        }
        let item = self
            .line
            .char_indices()
            .skip(self.cursor + 1)
            // Get into a word
            .skip_while(predicate)
            // Find when we are back out of the word
            .find(predicate);
        if let Some((index, _)) = item {
            index - 1
        } else {
            self.line.len() - 1
        }
    }

    fn get_current_word_start(&self) -> usize {
        fn predicate(item: &(usize, char)) -> bool {
            !item.1.is_alphanumeric()
        }
        let item = self
            .line
            .char_indices()
            .rev()
            .skip(self.line.len() - self.cursor)
            // Get into a word
            .skip_while(predicate)
            // Find when we are back out of the word
            .find(predicate);
        if let Some((index, _)) = item {
            index + 1
        } else {
            0
        }
    }

    fn get_next_word_start(&self) -> usize {
        fn predicate(item: &(usize, char)) -> bool {
            item.1.is_alphanumeric()
        }
        let item = self
            .line
            .char_indices()
            .skip(self.cursor)
            // Get out of a word
            .skip_while(predicate)
            // Find when we are back in a word
            .find(predicate);
        if let Some((index, _)) = item {
            index
        } else {
            self.line.len() - 1
        }
    }

    fn delete_word(&mut self, adjustment: OpAdjust) {
        let start = if adjustment == OpAdjust::Empty {
            self.cursor
        } else {
            self.get_current_word_start()
        };

        let end = if adjustment == OpAdjust::Inner {
            self.get_current_word_end()
        } else {
            let word_start = self.get_next_word_start();
            if word_start == self.line.len() - 1 {
                word_start
            } else {
                word_start - 1
            }
        };
        self.delete_range(start, end + 1);
    }

    fn delete_range(&mut self, start: usize, end: usize) {
        self.line.replace_range(start..end, "");
        self.cursor = start;
    }

    fn delete_current_char(&mut self) {
        self.line.remove(self.cursor);
    }

    fn delete_all(&mut self) {
        self.line = String::new();
        self.cursor = 0;
    }

    fn insert_char(&mut self, c: char) {
        if self.cursor < self.max_cursor() {
            self.line.insert(self.cursor, c);
        } else {
            self.line.push(c);
        }
    }
}

fn determine_cursor_shape(state: &PromptState) -> cursor::SetCursorStyle {
    match state.mode {
        PromptMode::Normal | PromptMode::OperatorPending(_) => cursor::SetCursorStyle::SteadyBlock,
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
        (mode, keycode, KeyModifiers::NONE | KeyModifiers::SHIFT) => match (mode, keycode) {
            (_, KeyCode::Enter) => {
                return Ok(true);
            }
            (PromptMode::Insert, KeyCode::Esc) => {
                state.normal_mode();
                state.move_left();
            }
            (PromptMode::Normal, KeyCode::Backspace) => state.move_left(),
            (PromptMode::Insert, KeyCode::Backspace) => {
                if state.cursor < state.max_cursor() {
                    if state.cursor != 0 {
                        state.move_left();
                        state.delete_current_char()
                    }
                } else if state.line.pop().is_some() {
                    state.move_left();
                }
            }
            (PromptMode::Insert, KeyCode::Char(c)) => {
                state.insert_char(c);
                state.move_right();
            }
            (PromptMode::Normal, KeyCode::Char(c)) => match c {
                'i' => state.insert_mode(),
                'I' => {
                    state.insert_mode();
                    state.move_to_start();
                }
                'a' => {
                    state.insert_mode();
                    state.move_right();
                }
                'A' => {
                    state.insert_mode();
                    state.move_to_end();
                }
                'x' => state.delete_current_char(),
                'X' => {
                    state.move_left();
                    state.delete_current_char();
                }
                'h' => state.move_left(),
                'l' => state.move_right(),
                'c' => state.operator_pending_mode(Operation::Change(OpAdjust::Empty)),
                'd' => state.operator_pending_mode(Operation::Delete(OpAdjust::Empty)),
                'e' => state.move_to_current_word_end(),
                'b' => state.move_to_current_word_start(),
                'w' => state.move_to_next_word_start(),
                _ => {}
            },
            (PromptMode::OperatorPending(operation), KeyCode::Char(c)) => match (operation, c) {
                (Operation::Change(OpAdjust::Empty), 'i') => {
                    state.operator_pending_mode(Operation::Change(OpAdjust::Inner))
                }
                (Operation::Change(OpAdjust::Empty), 'a') => {
                    state.operator_pending_mode(Operation::Change(OpAdjust::Around))
                }
                (Operation::Delete(OpAdjust::Empty), 'i') => {
                    state.operator_pending_mode(Operation::Delete(OpAdjust::Inner))
                }
                (Operation::Delete(OpAdjust::Empty), 'a') => {
                    state.operator_pending_mode(Operation::Delete(OpAdjust::Around));
                }
                (Operation::Change(adjustment), 'w') => {
                    state.delete_word(adjustment.clone());
                    state.insert_mode();
                }
                (Operation::Delete(adjustment), 'w') => {
                    state.delete_word(adjustment.clone());
                    state.normal_mode();
                }
                (Operation::Change(OpAdjust::Empty), 'e') => {
                    let end = state.get_current_word_end();
                    state.delete_range(state.cursor, end + 1);
                    state.insert_mode();
                }
                (Operation::Delete(OpAdjust::Empty), 'e') => {
                    let end = state.get_current_word_end();
                    state.delete_range(state.cursor, end + 1);
                    state.normal_mode();
                }
                (Operation::Change(OpAdjust::Empty), 'b') => {
                    let start = state.get_current_word_start();
                    state.delete_range(start, state.cursor);
                    state.insert_mode();
                }
                (Operation::Delete(OpAdjust::Empty), 'b') => {
                    let start = state.get_current_word_start();
                    state.delete_range(start, state.cursor);
                    state.normal_mode();
                }
                (Operation::Change(OpAdjust::Empty), 'c') => {
                    state.delete_all();
                    state.insert_mode();
                }
                (Operation::Delete(OpAdjust::Empty), 'd') => {
                    state.delete_all();
                    state.normal_mode();
                }
                (_, _) => state.normal_mode(),
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
    first_item: u16,
    items_shown: u16,
    max_index: u16,
    has_options: bool,
    prompt_state: PromptState,
}

impl SelectionState {
    fn new(items_shown: u16, input_start: u16, input_row: u16, max_index: u16) -> Self {
        SelectionState {
            selected: 0,
            first_item: 0,
            items_shown,
            max_index,
            has_options: true,
            prompt_state: PromptState::new(input_start, input_row),
        }
    }

    fn update_max_index(&mut self, max_index: u16, has_options: bool) {
        self.has_options = has_options;
        self.max_index = max_index;
        if self.selected > self.max_index {
            self.selected = self.max_index;
        }

        if self.first_item + self.items_shown > self.max_index {
            // + 1 is to account for 0 based index of max index
            if self.items_shown + 1 > self.max_index {
                self.first_item = 0;
            } else {
                self.first_item = self.max_index - self.items_shown + 1;
            }
        }
    }

    fn next_item(&mut self) {
        if self.selected < self.max_index {
            self.selected += 1;
            // - 2 is so the next item is shown and 0 based indexing
            if self.first_item + self.items_shown - 2 < self.selected
                && self.first_item < self.max_index
            {
                self.first_item += 1
            }
        }
    }

    fn previous_item(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            // + 1 is so the previous item is shown
            if self.first_item + 1 > self.selected && self.first_item > 0 {
                self.first_item -= 1
            }
        }
    }
}

fn select_handle_key(
    state: &mut SelectionState,
    key: KeyCode,
    modifiers: KeyModifiers,
) -> anyhow::Result<bool> {
    match (&state.prompt_state.mode, key, modifiers) {
        (_, KeyCode::Enter, KeyModifiers::NONE) => {
            if state.has_options {
                return Ok(true);
            }
        }
        (PromptMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE) => state.next_item(),
        (PromptMode::Normal, KeyCode::Char('k'), KeyModifiers::NONE) => state.previous_item(),
        (PromptMode::Insert, KeyCode::Char('n'), KeyModifiers::CONTROL) => state.next_item(),
        (PromptMode::Insert, KeyCode::Char('p'), KeyModifiers::CONTROL) => state.previous_item(),
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
    let first_item = usize::from(state.first_item);
    for (i, option) in options
        .iter()
        .skip(first_item)
        .take(MAX_OPTIONS_SHOWN)
        .enumerate()
    {
        if i > 0 {
            stderr.queue(cursor::MoveToNextLine(1))?;
        }
        // i is the index of the displayed items, but selected is the
        // index of the selected option in the list of all options add
        // first_item to reconcile that
        if i + first_item == selected_usize {
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
    }
    stderr
        .queue(style::SetForegroundColor(Color::Reset))?
        .queue(style::SetAttribute(style::Attribute::Reset))?;
    Ok(())
}

fn calculate_match_score(
    option: &str,
    filter_terms: &[&str],
    matcher: &SkimMatcherV2,
) -> Option<i64> {
    let mut score = 0;
    for term in filter_terms {
        match matcher.fuzzy_match(option, term) {
            Some(term_score) => score += term_score,
            None => return None,
        }
    }
    Some(score)
}

fn filter_options<'a>(filter: &str, options: &'a [String]) -> Vec<&'a String> {
    let filter_terms: Vec<&str> = filter.split_whitespace().collect();
    let matcher = SkimMatcherV2::default().smart_case();
    let mut matched: Vec<(i64, &String)> = options
        .iter()
        .filter_map(|option| {
            calculate_match_score(option, &filter_terms, &matcher).map(|score| (-score, option))
        })
        .collect();
    matched.sort();
    matched.into_iter().map(|(_, option)| option).collect()
}

pub fn select_prompt<'a>(prompt: &str, options: &'a [String]) -> anyhow::Result<&'a str> {
    let mut stderr = io::stderr();
    eprint!("{} ", prompt);
    stderr.flush()?;

    let items_shown = 10.min(options.len());
    let input_start = u16::try_from(prompt.len() + 1)?;
    let max_items = u16::try_from(options.len())? - 1;
    let mut state = SelectionState::new(u16::try_from(items_shown)?, input_start, 0, max_items);

    enable_raw_mode()?;

    // Make room for the options to be printed and return to input line
    eprint!("{}", "\n".repeat(items_shown));
    stderr.queue(cursor::MoveUp(state.items_shown))?;

    let (_, position_row) = cursor::position()?;
    // Move from prompt to first line of options
    stderr.queue(cursor::MoveToNextLine(1))?;
    print_options(&state, &options.iter().collect(), &mut stderr)?;
    state.prompt_state.input_row = position_row;
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
        if filtered_options.is_empty() {
            state.update_max_index(0, false);
        } else {
            let new_num_items = u16::try_from(filtered_options.len()).unwrap_or(state.items_shown);
            state.update_max_index(new_num_items - 1, true);
        }

        print_prompt_input(&state.prompt_state, &mut stderr)?;
        stderr.queue(cursor::MoveToNextLine(1))?;
        print_options(&state, &filtered_options, &mut stderr)?;
        update_cursor(&state.prompt_state, &mut stderr)?;
        stderr.flush()?;
    }
    stderr
        .queue(cursor::MoveTo(
            state.prompt_state.input_start,
            state.prompt_state.input_row,
        ))?
        .queue(Clear(ClearType::FromCursorDown))?
        .flush()?;
    disable_raw_mode()?;

    let filtered_options = filter_options(&state.prompt_state.line, options);
    let result = filtered_options[usize::from(state.selected)];
    let result_output = format!("{}\n", &result);
    stderr.execute(PrintStyledContent(result_output.with(Color::Cyan)))?;
    Ok(result)
}

fn print_boolean_toogle(state: bool, stderr: &mut dyn Write) -> anyhow::Result<()> {
    if state {
        stderr
            .queue(style::PrintStyledContent(
                " y ".on(Color::DarkGreen).attribute(Attribute::Bold),
            ))?
            .queue(style::Print(" | "))?
            .queue(style::PrintStyledContent(" n ".attribute(Attribute::Dim)))?;
    } else {
        stderr
            .queue(style::PrintStyledContent(" y ".attribute(Attribute::Dim)))?
            .queue(style::Print(" | "))?
            .queue(style::PrintStyledContent(
                " n ".on(Color::Red).attribute(Attribute::Bold),
            ))?;
    }
    Ok(())
}

pub fn boolean_prompt(prompt: &str, default: bool) -> anyhow::Result<bool> {
    let mut stderr = io::stderr();
    let mut state = default;

    eprint!("{} ", prompt);

    enable_raw_mode()?;
    stderr.queue(cursor::SavePosition)?.queue(cursor::Hide)?;
    print_boolean_toogle(state, &mut stderr)?;
    stderr.flush()?;

    while let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event::read()?
    {
        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                bail!("ctrl-c sent");
            }
            (code, KeyModifiers::NONE | KeyModifiers::SHIFT) => match code {
                KeyCode::Enter => {
                    break;
                }
                KeyCode::Char('l' | 'f' | 'n') => {
                    state = false;
                }
                KeyCode::Char('h' | 't' | 'y') => {
                    state = true;
                }
                _ => {}
            },
            _ => {}
        }
        stderr.queue(cursor::RestorePosition)?;
        print_boolean_toogle(state, &mut stderr)?;
        stderr.flush()?;
    }

    stderr.execute(cursor::Show)?;
    disable_raw_mode()?;
    eprintln!();
    Ok(state)
}
