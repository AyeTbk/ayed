use ayed_regex::Regex;

fn main() {
    let re = Regex::new(r"h(e|a)llo .*|bye my guy").unwrap();

    dbg!(re.is_match("ab"));
    dbg!(re.is_match("abc"));
    dbg!(re.is_match("bc"));
    dbg!(re.is_match("c"));
    dbg!(re.is_match("hallo wereld?"));
    dbg!(re.is_match("hello world!"));
    dbg!(re.is_match("bye my guy"));
}
