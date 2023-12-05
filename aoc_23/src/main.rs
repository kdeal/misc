use std::env;
use std::fs;

mod day1;
mod day2;

fn main() {
    let args: Vec<String> = env::args().collect();
    let problem = &args[1];
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result = match problem.as_str() {
        "1a" => day1::problem_a(contents),
        "1b" => day1::problem_b(contents),
        "2a" => day2::problem_a(contents),
        "2b" => day2::problem_b(contents),
        &_ => panic!("Day not recognized"),
    };
    println!("{}", result);
}
