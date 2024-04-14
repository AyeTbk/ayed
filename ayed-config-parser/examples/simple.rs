use ayed_config_parser::parse_module;

pub fn main() {
    let _ = dbg!(parse_module(
        r#"
            file .*\.rs {
                syntax {
                    # hello there!
                    keyword \b(pub|use|mod)\b
                    literal \b(4)\b
                }
            }
        "#
    ));
}
