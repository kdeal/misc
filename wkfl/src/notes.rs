use std::time::SystemTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

use crate::utils::to_title_case;

pub enum DailyNoteSpecifier {
    Yesterday,
    Today,
    Tomorrow,
}

pub enum NoteSpecifier {
    Daily { day: DailyNoteSpecifier },
    Topic { name: String },
    Person { who: String },
}

const DAILY_NOTE_FORMAT: &[BorrowedFormatItem] = format_description!("daily/[year repr:full]/[week_number repr:sunday]/[weekday repr:short]_[month repr:short]_[day].md");
const DAILY_NOTE_TITLE_FORMAT: &[BorrowedFormatItem] =
    format_description!("[weekday repr:long] [month repr:long] [day padding:none]");

fn get_day_suffix<'a>(day: u8) -> &'a str {
    match day {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    }
}

fn get_path_for_topic(topic_name: &str) -> String {
    let name_in_path = topic_name
        .to_lowercase()
        .replace(" ", "_")
        .replace("-", "_");
    format!("topics/{name_in_path}.md")
}

fn get_path_for_person(topic_name: &str) -> String {
    let name_in_path = topic_name
        .to_lowercase()
        .replace(" ", "_")
        .replace("-", "_");
    format!("people/{name_in_path}.md")
}

fn date_from_note_specifier(note_specifier: &DailyNoteSpecifier) -> Date {
    let cur_time: OffsetDateTime = SystemTime::now().into();
    let cur_date: Date = cur_time.date();
    match note_specifier {
        // Current date isn't going to be min date
        DailyNoteSpecifier::Yesterday => cur_date.previous_day().unwrap(),
        DailyNoteSpecifier::Today => cur_date,
        // Current date isn't going to be max date
        DailyNoteSpecifier::Tomorrow => cur_date.next_day().unwrap(),
    }
}

pub fn format_note_path(note_specifier: &NoteSpecifier) -> String {
    match note_specifier {
        NoteSpecifier::Topic { name } => get_path_for_topic(name),
        NoteSpecifier::Daily { day } => date_from_note_specifier(day)
            .format(DAILY_NOTE_FORMAT)
            .unwrap(),
        NoteSpecifier::Person { who } => get_path_for_person(who),
    }
}

pub fn note_template(note_specifier: &NoteSpecifier) -> String {
    match note_specifier {
        NoteSpecifier::Daily { day } => {
            let date = date_from_note_specifier(day);
            let date_str = date.format(DAILY_NOTE_TITLE_FORMAT).unwrap();
            let day_suffix = get_day_suffix(date.day());
            format!("# {date_str}{day_suffix}\n\n## ")
        }
        NoteSpecifier::Topic { name } => format!("# {}", to_title_case(name)),
        NoteSpecifier::Person { who } => format!("# {who}"),
    }
}
