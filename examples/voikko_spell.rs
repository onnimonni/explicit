//! Spell-check words from stdin with the embedded Finnish speller, one per line:
//! `C word` / `W word` (as `voikkospell`), with `-s` the suggestions of wrong words, with
//! `-a` the morphological readings.
//! Used to compare the port against libvoikko.

use std::io::BufRead;

fn main() {
    let suggest = std::env::args().any(|a| a == "-s");
    let analyze = std::env::args().any(|a| a == "-a");
    let v = explicit::voikko::embedded();
    for line in std::io::stdin().lock().lines() {
        let Ok(w) = line else { break };
        if analyze {
            println!("{w}: {:?}", v.analyses(&w));
            continue;
        }
        if v.spell(&w) {
            println!("C: {w}");
        } else if suggest {
            println!("W: {w}: {}", v.suggest(&w).join(", "));
        } else {
            println!("W: {w}");
        }
    }
}
