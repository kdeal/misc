use std::env;
use std::fs;

fn find_first_digit<I: Iterator<Item = char>>(mut chars: I) -> Option<u32> {
    chars.find(|c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap())
}

fn extract_calibration_numbers(line: &str) -> u32 {
    if let Some(first_digit) = find_first_digit(line.chars()) {
        let last_digit = find_first_digit(line.chars().rev()).unwrap();
        first_digit * 10 + last_digit
    } else {
        0
    }
}


fn day1_problem_a(args: Vec<String>) {
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result: u32 = contents
        .split('\n')
        .map(|line| extract_calibration_numbers(line))
        .sum();
    println!("{}", result);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let problem = &args[1];
    match problem.as_str() {
        "1a" => day1_problem_a(args),
        &_ => (),
    };
}
