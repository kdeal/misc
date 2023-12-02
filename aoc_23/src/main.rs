use std::env;

mod day1;

fn main() {
    let args: Vec<String> = env::args().collect();
    let problem = &args[1];
    match problem.as_str() {
        "1a" => day1::problem_a(args),
        "1b" => day1::problem_b(args),
        &_ => (),
    };
}
