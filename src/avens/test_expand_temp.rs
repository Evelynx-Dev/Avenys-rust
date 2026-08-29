// This will be compiled as a test within the avens crate
#[test]
fn test_expand_derives_source() {
    use crate::derive::expand_derives_source;
    let source = r#"load mire::str
@[derive(Clone, PartialEq)]
struct Point {
    x :i64
    label :str
}
pub fn main: () {
    set a = (Point x: 1, label: "A")
    set c = a.clone()
    dasu(str::from::bool(a.equals(c)))
}
"#;
    let expanded = expand_derives_source(source);
    eprintln!("EXPANDED:\n{}", expanded);
}
