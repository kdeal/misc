use std::fs;

#[derive(Default, Debug)]
struct Cubes {
    blue: u32,
    red: u32,
    green: u32,
}

#[derive(Debug)]
struct Game {
    id: u32,
    rounds: Vec<Cubes>,
}

fn parse_round(round_str: &str) -> Cubes {
    let mut round = Cubes::default();
    round_str.split(',').for_each(|color_str| {
        let (number_str, color) = color_str.trim().split_once(" ").unwrap();
        let number = number_str.trim().parse::<u32>().unwrap();
        match color {
            "red" => round.red = number,
            "blue" => round.blue = number,
            "green" => round.green = number,
            _ => panic!("Unrecognized color"),
        };
    });
    round
}

fn parse_game(line: &str) -> Option<Game> {
    if line.is_empty() {
        return None;
    }

    let (game_str, rounds_str) = line.split_once(":").unwrap();
    let game_id = game_str
        .strip_prefix("Game ")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let rounds = rounds_str
        .split(';')
        .map(|round_str| parse_round(round_str))
        .collect();
    Some(Game {
        id: game_id,
        rounds,
    })
}

pub fn problem_a(args: Vec<String>) {
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let games: Vec<Game> = contents
        .split('\n')
        .filter_map(|line| parse_game(line))
        .collect();
    let bag_cubes = Cubes {
        blue: 14,
        red: 12,
        green: 13,
    };
    let valid_games: u32 = games
        .iter()
        .filter(|game| {
            game.rounds.iter().all(|round| {
                round.blue <= bag_cubes.blue
                    && round.red <= bag_cubes.red
                    && round.green <= bag_cubes.green
            })
        })
        .map(|game| game.id)
        .sum();
    println!("{:?}", valid_games);
}

fn calculate_cube_power(game: Game) -> u32 {
    let blue_max = game.rounds.iter().map(|round| round.blue).max().unwrap();
    let red_max = game.rounds.iter().map(|round| round.red).max().unwrap();
    let green_max = game.rounds.iter().map(|round| round.green).max().unwrap();
    blue_max * green_max * red_max
}

pub fn problem_b(args: Vec<String>) {
    let file_path = &args[2];
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let result: u32 = contents
        .split('\n')
        .filter_map(|line| parse_game(line))
        .map(|game| calculate_cube_power(game))
        .sum();
    println!("{}", result);
}
