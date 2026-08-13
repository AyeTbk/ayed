use ayed_glob::Glob;

fn main() {
    let g = Glob::new("**/*ic");
    dbg!(&g);
    dbg!(g.is_match("haha/awd/aas/ebcdic"));
}
