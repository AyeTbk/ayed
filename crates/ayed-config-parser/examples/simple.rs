use ayed_config_parser::parse_module;

pub fn main() {
    let _ = dbg!(parse_module(
        r#"
            file .*\.rs {
                syntax {
                    # hello there!
                    keyword \b(pub|use|mod)\b
                    literal \b(4)\b
                    list  $[ wow ]
                    list2 $[ a ; amazing ; ]
                    list3 $[
                            a funny dog ;
                            amazing ;
                        ]
                    #esc   ive got 5$$
                    str $"hello!  "
                    str2 what$" are "you
                    str3 what $" are " you
                    #strlist $[ $"$1" ; tw$"o" ; $"thr
                    #                                ee" ]
                }
            }
        "#
    ));
}
