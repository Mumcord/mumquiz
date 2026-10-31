//! High scores: the same table the upstream bot kept, tested on the file it really uses.

use mumquiz::scores::{MemoryScores, Scores, SqliteScores, TOP_LIMIT};

#[test]
fn a_win_is_added_to_the_best_score() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    let scores = SqliteScores::open(file.path()).expect("open");
    assert_eq!(scores.wins("barbara"), None);
    let mut scores = scores;
    scores.add_win("barbara").expect("win");
    scores.add_win("barbara").expect("win");
    scores.add_win("zorg").expect("win");
    assert_eq!(scores.wins("barbara"), Some(2));
    assert_eq!(
        scores.top(TOP_LIMIT).expect("top"),
        vec![("barbara".to_owned(), 2), ("zorg".to_owned(), 1)]
    );
    assert_eq!(
        scores.alltime().expect("alltime"),
        Some(("barbara".to_owned(), 2))
    );
}

#[test]
fn a_renamed_quizzer_keeps_the_wins() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    let mut scores = SqliteScores::open(file.path()).expect("open");
    scores.add_win("barbara").expect("win");
    assert!(scores.rename("barbara", "hans").expect("rename"));
    assert_eq!(scores.wins("hans"), Some(1));
    assert_eq!(scores.wins("barbara"), None);
    assert!(!scores.rename("barbara", "hans").expect("no entry"));
}

#[test]
fn memory_and_sqlite_agree() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    let mut sqlite = SqliteScores::open(file.path()).expect("open");
    let mut memory = MemoryScores::new();
    for name in ["barbara", "zorg", "zorg"] {
        sqlite.add_win(name).expect("win");
        memory.add_win(name).expect("win");
    }
    assert_eq!(
        sqlite.top(TOP_LIMIT).expect("top"),
        memory.top(TOP_LIMIT).expect("top")
    );
}
