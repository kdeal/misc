use std::collections::HashMap;

struct Card {
    id: u32,
    winning_numbers: Vec<u32>,
    card_numbers: Vec<u32>,
    num_matching: u32,
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
    let winning_numbers = parse_numbers(winning_numbers_str);
    let card_numbers = parse_numbers(card_number_str);
    let usize_matching = card_numbers
        .iter()
        .filter(|number| winning_numbers.contains(&number))
        .count();
    Card {
        id: card_id,
        winning_numbers,
        card_numbers,
        num_matching: u32::try_from(usize_matching).unwrap(),
    }
}

fn calculate_card_score(card: Card) -> u32 {
    if card.num_matching == 0 {
        return 0;
    }
    let base: i32 = 2;
    let result = base.pow(card.num_matching - 1);
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

pub fn problem_b(contents: String) -> u32 {
    let cards = contents
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| parse_card(line));

    let card_num_to_card: HashMap<u32, Card> =
        cards.into_iter().map(|card| (card.id, card)).collect();

    let mut num_cards = 0;
    let mut cards_to_process: Vec<u32> = card_num_to_card
        .keys()
        .map(|card_id| card_id.clone())
        .collect();
    while let Some(card_id) = cards_to_process.pop() {
        num_cards += 1;
        let card = card_num_to_card.get(&card_id).unwrap();
        let num_card_matches = card.num_matching;
        for copy_offset in 0..num_card_matches {
            cards_to_process.push(card_id + 1 + copy_offset);
        }
    }
    num_cards
}
