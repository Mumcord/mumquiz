//! Answer matching: what a player may type, tested with the accents and punctuation real chat has.

use mumquiz::answer;

#[test]
fn it_matches_case_and_punctuation() {
    assert!(answer::matches(&["Merkur".to_owned()], "is it merkur?"));
    assert!(answer::matches(&["Merkur".to_owned()], "MERKUR"));
    assert!(answer::matches(
        &["Mercury".to_owned()],
        "I think it is mercury, yes"
    ));
}

#[test]
fn it_matches_accents() {
    assert!(answer::matches(&["Korona".to_owned()], "die KOERONA"));
    assert!(answer::matches(&["Über".to_owned()], "ueber"));
    assert!(answer::matches(&["Straße".to_owned()], "strasse"));
}

#[test]
fn a_short_answer_needs_its_own_word() {
    let rot = vec!["rot".to_owned()];
    assert!(answer::matches(&rot, "der Planet ist rot"));
    assert!(!answer::matches(&rot, "proton"));
}

#[test]
fn every_accepted_answer_counts() {
    let answers = vec!["Sonne".to_owned(), "Sol".to_owned()];
    assert!(answer::matches(&answers, "klar, sol"));
    assert!(!answer::matches(&answers, "mond"));
}

#[test]
fn an_empty_answer_matches_nothing() {
    assert!(!answer::matches(&["".to_owned()], "hallo"));
}

#[test]
fn normalize_keeps_words_readable() {
    assert_eq!(answer::normalize("  Jupiter?!  "), "jupiter");
    assert_eq!(answer::normalize("Alpha Centauri"), "alpha centauri");
}
