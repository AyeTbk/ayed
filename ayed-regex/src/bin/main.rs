use ayed_regex::Regex;

fn main() {
    let re = Regex::new(r".*!").unwrap();

    // dbg!(re.is_match("ab"));
    // dbg!(re.is_match("abc"));
    // dbg!(re.is_match("bc"));
    // dbg!(re.is_match("c"));
    dbg!(re.is_match("a!"));
}
