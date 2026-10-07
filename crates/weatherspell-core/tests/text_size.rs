// The Mac's text size steps, from its system font's 13 points.

use weatherspell_core::text_size::TextSize;

const MAC: i32 = 13;

fn walk(from: TextSize, step: fn(TextSize) -> TextSize) -> Vec<i32> {
    let mut sizes = vec![from.points(MAC)];
    let mut size = from;
    loop {
        let next = step(size);
        if next == size {
            return sizes;
        }
        size = next;
        sizes.push(size.points(MAC));
    }
}

#[test]
fn bigger_steps_up_to_three_times() {
    assert_eq!(
        walk(TextSize::ACTUAL, TextSize::bigger),
        [13, 14, 16, 20, 23, 26, 33, 39]
    );
}

#[test]
fn smaller_steps_twice() {
    assert_eq!(walk(TextSize::ACTUAL, TextSize::smaller), [13, 12, 10]);
}

#[test]
fn actual_size_is_the_system_size() {
    assert_eq!(TextSize::default(), TextSize::ACTUAL);
    assert_eq!(TextSize::ACTUAL.percent(), 100);
    assert_eq!(TextSize::ACTUAL.points(MAC), MAC);
}

#[test]
fn every_step_changes_the_size() {
    let mut sizes = walk(TextSize::ACTUAL, TextSize::smaller);
    sizes.reverse();
    sizes.extend(walk(TextSize::ACTUAL.bigger(), TextSize::bigger));
    assert!(sizes.windows(2).all(|pair| pair[0] < pair[1]), "{sizes:?}");
}
