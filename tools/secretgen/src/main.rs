use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

fn main() {
    let mut rng = thread_rng();
    let s: String = (0..12)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();
    print!("{}", s);
}
