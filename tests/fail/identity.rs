use gargoyle::with_guile;

fn main() {
    with_guile(std::convert::identity);
}
