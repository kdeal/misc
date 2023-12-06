struct Card {
    id: u32,
    winning_numbers: Vec<u32>,
    card_numbers: Vec<u32>,
}

fn parse_numbers(numbers_str: &str) -> Vec<u32> {
    numbers_str
        .split(" ")
        .filter(|str| !str.is_empty())
        .map(|number_str| number_str.trim().parse().unwrap())
        .collect()
}

fn parse_card(line: &str) -> Card {
    let (card_str, numbers_str) = line.split_once(":").unwrap();
    let card_id = card_str
        .strip_prefix("Card ")
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    let (card_number_str, winning_numbers_str) = numbers_str.split_once("|").unwrap();
    Card {
        id: card_id,
        winning_numbers: parse_numbers(winning_numbers_str),
        card_numbers: parse_numbers(card_number_str),
    }
}

fn calculate_card_score(card: Card) -> u32 {
    let num_matching = card
        .card_numbers
        .iter()
        .filter(|number| card.winning_numbers.contains(&number))
        .count();
    if num_matching == 0 {
        return 0;
    }
    let base: i32 = 2;
    let result = base.pow(u32::try_from(num_matching - 1).unwrap());
    result.try_into().unwrap()
}

pub fn problem_a(contents: String) -> u32 {
    contents
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| parse_card(line))
        .map(|card| calculate_card_score(card))
        .sum()
}
