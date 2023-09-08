use ayed_regex::Regex;

fn main() {
    let re = Regex::new(r"h(e|a)llo (.*)(!|.)").unwrap();

    dbg!(re.is_match("hello world!"));
    dbg!(re.is_match("hallo wereld?"));
    dbg!(re.is_match("bye my guy"));
}
