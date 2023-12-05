use std::fs;

const NUMBERS_SPELLED: &'static [(&str, u32)] = &[
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
];

fn find_first_digit<I: Iterator<Item = char>>(mut chars: I) -> Option<u32> {
    chars
        .find(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
}

fn extract_calibration_numbers(line: &str) -> u32 {
    if let Some(first_digit) = find_first_digit(line.chars()) {
        let last_digit = find_first_digit(line.chars().rev()).unwrap();
        first_digit * 10 + last_digit
    } else {
        0
    }
}

pub fn problem_a(args: Vec<String>) {
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result: u32 = contents
        .split('\n')
        .map(|line| extract_calibration_numbers(line))
        .sum();
    println!("{}", result);
}

fn parse_spelled_digit(string: String, reversed: bool) -> Option<u32> {
    for (number_str, value) in NUMBERS_SPELLED {
        if reversed {
            let rev_number_str: String = number_str.chars().rev().collect();
            if string.ends_with(&rev_number_str) {
                return Some(value.clone());
            }
        } else {
            if string.ends_with(number_str) {
                return Some(value.clone());
            }
        }
    }
    None
}

fn first_digit_or_spelled_digit<I: Iterator<Item = char>>(chars: I, reversed: bool) -> Option<u32> {
    let mut accum_str = String::new();
    for char in chars {
        if char.is_ascii_digit() {
            return Some(char.to_digit(10).unwrap());
        }
        accum_str.push(char);
        if let Some(digit) = parse_spelled_digit(accum_str.clone(), reversed) {
            return Some(digit);
        }
    }
    None
}

fn extract_calibration_numbers_digit_and_spelled_digit(line: &str) -> u32 {
    if let Some(first_digit) = first_digit_or_spelled_digit(line.chars(), false) {
        let last_digit = first_digit_or_spelled_digit(line.chars().rev(), true).unwrap();
        let value = first_digit * 10 + last_digit;
        println!("{}: {}", line, value);
        value
    } else {
        0
    }
}

pub fn problem_b(args: Vec<String>) {
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result: u32 = contents
        .split('\n')
        .map(|line| extract_calibration_numbers_digit_and_spelled_digit(line))
        .sum();
    println!("{}", result);
}
