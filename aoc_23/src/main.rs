use std::env;
use std::fs;

fn extract_calibration_numbers(line: &str) -> u32 {
    eprintln!("line: {}", line);
    if let Some(first_digit_str) = line.chars().find(|c| c.is_ascii_digit()) {
        let first_digit = first_digit_str.to_digit(10).unwrap();
        let last_digit = line
            .chars()
            .rev()
            .find(|c| c.is_ascii_digit())
            .unwrap()
            .to_digit(10)
            .unwrap();
        first_digit * 10 + last_digit
    } else {
        0
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let problem = &args[1];
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result: u32 = contents
        .split('\n')
        .map(|line| extract_calibration_numbers(line))
        .sum();
    println!("{}", result);
}
