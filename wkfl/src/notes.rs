use std::time::SystemTime;
use time::error;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

pub enum DailyNoteSpecifier {
    Yesterday,
    Today,
    Tomorrow,
}

const DAILY_NOTE_FORMAT: &[BorrowedFormatItem] = format_description!("daily/[year repr:last_two]/[week_number repr:sunday]/[weekday repr:short]_[month repr:short]_[day].md");

pub fn format_note_path(note_specifier: DailyNoteSpecifier) -> Result<String, error::Format> {
    let cur_time: OffsetDateTime = SystemTime::now().into();
    let cur_date: Date = cur_time.date();
    let date = match note_specifier {
        // Current date isn't going to be min date
        DailyNoteSpecifier::Yesterday => cur_date.previous_day().unwrap(),
        DailyNoteSpecifier::Today => cur_date,
        // Current date isn't going to be max date
        DailyNoteSpecifier::Tomorrow => cur_date.next_day().unwrap(),
    };
    date.format(DAILY_NOTE_FORMAT)
}
