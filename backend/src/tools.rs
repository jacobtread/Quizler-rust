use rand::{Rng, thread_rng};

pub type Identifier = String;

const IDENTIFIER_CHARS: &'static [char; 16] = &['A', 'B', 'C', 'D', 'E', 'F', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

pub fn random_identifier(length: usize) -> Identifier {
    let mut out = String::with_capacity(length);
    let mut rand = thread_rng();
    for _ in 0..length {
        let ran = rand.gen_range(0..16);
        out.push(IDENTIFIER_CHARS[ran])
    }
    out
}
