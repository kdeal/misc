use std::collections::{HashSet, HashMap};

#[derive(Default, Debug, Eq, PartialEq, Hash)]
struct Point {
    row: i32,
    column: i32,
}

#[derive(Debug)]
struct Part {
    end_point: Point,
    length: i32,
    value: u32,
}

fn is_symbol(char: char) -> bool {
    !(char.is_ascii_alphanumeric() || char == '.')
}

fn parse_schematic(contents: String) -> (HashSet<Point>, Vec<Part>, Vec<Point>) {
    let mut point_to_is_symbol = HashSet::new();
    let mut gear_points = vec![];
    let mut part_numbers = vec![];

    contents.split("\n").enumerate().for_each(|(u_row, line)| {
        let row = i32::try_from(u_row).unwrap();
        let mut cur_number = String::new();
        line.chars().enumerate().for_each(|(u_column, char)| {
            let column = i32::try_from(u_column).unwrap();

            if char.is_ascii_digit() {
                cur_number.push(char);
            } else if !cur_number.is_empty() {
                let point = Point {row, column: column - 1};
                let part = Part { end_point: point, length: i32::try_from(cur_number.len()).unwrap(), value: cur_number.parse().unwrap() };
                part_numbers.push(part);
                cur_number = String::new();
            }
            if is_symbol(char) {
                if char == '*' {
                    gear_points.push(Point {row, column})
                }
                point_to_is_symbol.insert(Point {row, column});
            }
        });

        if !cur_number.is_empty() {
            let end_point = Point {row, column: i32::try_from(line.len()).unwrap() - 1 };
            let part = Part { end_point, length: i32::try_from(cur_number.len()).unwrap(), value: cur_number.parse().unwrap() };
            part_numbers.push(part);
        }
    });

    (point_to_is_symbol, part_numbers, gear_points)
}

fn check_part_number(part_number: &Part, symbol_points: &HashSet<Point>) -> Vec<Point> {
    let mut matched_points = vec![];
    let column_start = part_number.end_point.column - part_number.length;
    let column_end = part_number.end_point.column + 1;

    for column in column_start..=column_end {
        let above_point = Point {row: part_number.end_point.row - 1, column};
        if symbol_points.contains(&above_point){
            matched_points.push(above_point);
        }
        let below_point = Point {row: part_number.end_point.row + 1, column};
        if symbol_points.contains(&below_point) {
            matched_points.push(below_point);
        }
    }

    let before_point = Point {row: part_number.end_point.row, column: column_start };
    if symbol_points.contains(&before_point) {
        matched_points.push(before_point);
    }

    let after_point = Point {row: part_number.end_point.row, column: column_end };
    if symbol_points.contains(&after_point) {
        matched_points.push(after_point);
    }
    return matched_points;
}

pub fn problem_a(contents: String) -> u32 {
    let (symbol_points, part_numbers, _) = parse_schematic(contents);
    let valid_part_numbers: Vec<Part> = part_numbers.into_iter().filter(|part_number| !check_part_number(part_number, &symbol_points).is_empty()).collect();
    valid_part_numbers.iter().map(|part_number| part_number.value).sum()
}

pub fn problem_b(contents: String) -> u32 {
    let (symbol_points, part_numbers, gear_points) = parse_schematic(contents);
    let part_points: Vec<(Vec<Point>, Part)> = part_numbers.into_iter().map(|part_number| (check_part_number(&part_number, &symbol_points), part_number)).collect();

    let mut point_to_parts = HashMap::new();
    part_points.iter().for_each(|(points, part)| points.iter().for_each(|point| {
        if !point_to_parts.contains_key(&point) {
            point_to_parts.insert(point, Vec::new());
        }
        let part_list = point_to_parts.get_mut(point).unwrap();
        part_list.push(part.value.clone());
    }));

    gear_points.iter().filter_map(|gear_point| {
        if !point_to_parts.contains_key(gear_point) {
            return None
        }
        let part_numbers = point_to_parts.get(gear_point).unwrap();
        if part_numbers.len() != 2 {
            return None;
        }
        Some(part_numbers[0] * part_numbers[1])
    }).sum()
}
