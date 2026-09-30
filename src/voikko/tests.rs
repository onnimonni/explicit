use super::*;

fn v() -> &'static Voikko {
    embedded()
}

#[test]
fn correct_finnish_passes() {
    for w in [
        "kissa",
        "Kissa",
        "KISSA",
        "talossa",
        "taloissammekin",
        "potilasasiakirja",
        "potilasasiakirjojen",
        "tietojärjestelmä",
        "Helsinki",
        "Helsingissä",
        "esim.",
        "jne.",
        "ks.",
        "EU-maissa",
        "sähkö-",
        "ja-sana",
        "maa-alue",
        "käyttöönotto",
    ] {
        assert!(v().spell(w), "{w}");
    }
}

#[test]
fn misspellings_fail() {
    for w in [
        "kisssa",
        "helsinki",
        "talosa",
        "potilasasiakrja",
        "tietojärjestlmä",
        "maaalue",
        "hEl",
    ] {
        assert!(!v().spell(w), "{w}");
    }
}

#[test]
fn suggestions() {
    assert_eq!(
        v().suggest("kisssa").first().map(String::as_str),
        Some("kissa")
    );
    assert!(v().suggest("talosa").iter().any(|s| s == "talossa"));
    assert_eq!(
        v().suggest("helsinki").first().map(String::as_str),
        Some("Helsinki")
    );
}
