use {
    gargoyle::{with_guile, with_guile_protected},
    std::convert::identity,
};

fn main() {
    with_guile(identity);
    with_guile_protected(|l, _| l);
    with_guile_protected(|_, r| r);
}
