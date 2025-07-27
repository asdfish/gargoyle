use gargoyle::with_guile;

fn main() {
    with_guile(|api| {
        let t = api.make(true);
        api.without_guile(|| println!("{t:?}"));
    });

    with_guile(|api| {
        let t = api.make(true);
        api.without_guile(|| {});
        println!("{t:?}");
    });
}
