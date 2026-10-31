//! Game logic tests: the flow the upstream Python bot had, on an injected clock and random source.

use std::collections::BTreeMap;

use mumquiz::config::{Config, QuizRules, Server};
use mumquiz::question::Bank;
use mumquiz::quiz::{Action, Kind, Quiz};
use mumquiz::rng::Rng;
use mumquiz::scores::MemoryScores;
use mumquiz::strings::Strings;

fn rules() -> QuizRules {
    QuizRules {
        min_players: 2,
        hint_patience_secs: 20,
        stamina: 3,
        recycle_percent: 50,
        answer_gap_secs: 5,
        tick_secs: 1,
        waiting_secs: 30,
        command_prefix: "!".to_owned(),
        masters: vec!["Mumquiz".to_owned()],
        ..QuizRules::default()
    }
}

fn bank() -> Bank {
    Bank::parse(
        r#"
[[question]]
category = "Biologie"
question = "Aus was wird Opium gewonnen?"
answer = "Schlafmohn"

[[question]]
category = "Zahlen"
question = "Wie viele Beine hat eine Spinne?"
answer = "8"

[[question]]
category = "Sprache"
question = "Wie heisst unser Stern?"
answer = "a"
"#,
    )
    .expect("bank")
}

/// One question with a long answer: the game is predictable, so the tests can check hints and hunger.
fn long_bank() -> Bank {
    Bank::parse(
        r#"
[[question]]
category = "Biologie"
question = "Aus was wird Opium gewonnen?"
answer = "Schlafmohn"
"#,
    )
    .expect("bank")
}

fn game_with(bank: Bank) -> Quiz {
    let config = Config {
        server: Server {
            name: "Mumquiz".to_owned(),
            channel: Some("Quiz".to_owned()),
            ..Server::default()
        },
        quiz: rules(),
        tokens: Vec::new(),
        strings: BTreeMap::new(),
    };
    Quiz::new(
        &config,
        &Strings::default(),
        bank,
        Box::new(MemoryScores::new()),
        Rng::seeded(7),
    )
}

fn game() -> Quiz {
    game_with(bank())
}

fn kinds(actions: &[Action]) -> Vec<Kind> {
    actions
        .iter()
        .map(|action| match action {
            Action::Channel(line) | Action::Private { line, .. } => line.kind,
            Action::Topic(_) | Action::JoinChannel(_) | Action::ReloadQuestions => Kind::Info,
        })
        .collect()
}

fn texts(actions: &[Action]) -> Vec<String> {
    actions
        .iter()
        .map(|action| match action {
            Action::Channel(line) | Action::Private { line, .. } => line.text.clone(),
            Action::Topic(text) | Action::JoinChannel(text) => text.clone(),
            Action::ReloadQuestions => "reload".to_owned(),
        })
        .collect()
}

fn points(quiz: &Quiz, name: &str) -> Option<u64> {
    *quiz.quizzers().get(name)?
}

fn answer(quiz: &mut Quiz, name: &str, second: u64) -> Vec<Action> {
    let question = quiz.pending().expect("a question is open").answers[0].clone();
    quiz.message(name, &question, second).expect("answer")
}

#[test]
fn the_bot_itself_is_never_a_player() {
    let mut quiz = game();
    assert!(!quiz.add("Mumquiz"));
    assert!(quiz.add("barbara"));
    assert_eq!(quiz.quizzers().len(), 1);
    assert!(quiz.remove("barbara"));
    assert!(!quiz.remove("barbara"));
}

#[test]
fn it_waits_until_enough_players_are_there() {
    let mut quiz = game();
    quiz.add("barbara");
    let actions = quiz.tick(0);
    assert_eq!(kinds(&actions), [Kind::Info]);
    assert!(texts(&actions)[0].contains("wait"));
    assert!(quiz.tick(10).is_empty());
    assert!(!quiz.tick(30).is_empty());
    assert!(quiz.tick(40).is_empty());
    assert!(!quiz.tick(60).is_empty());
}

#[test]
fn the_first_tick_with_players_asks_a_question() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    let actions = quiz.tick(0);
    assert_eq!(kinds(&actions), [Kind::Question]);
    assert!(quiz.pending().is_some());
    // Two players: the target score is 1 + 2/2.
    assert_eq!(quiz.target_score(), 2);
}

#[test]
fn a_correct_answer_scores_once_and_the_target_score_wins() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);

    let actions = answer(&mut quiz, "barbara", 1);
    assert_eq!(kinds(&actions), [Kind::Correct]);
    assert_eq!(points(&quiz, "barbara"), Some(1));

    // The question is closed, so the same answer again gives nothing.
    assert!(
        quiz.message("barbara", "Schlafmohn", 2)
            .expect("answer")
            .is_empty()
    );

    // The second correct answer reaches the target score: winner, and the round resets.
    quiz.tick(6);
    let actions = answer(&mut quiz, "barbara", 7);
    assert_eq!(kinds(&actions), [Kind::Correct, Kind::Winner, Kind::Info]);
    assert!(points(&quiz, "barbara").is_none_or(|p| p == 0));
    assert!(quiz.pending().is_none());

    // The win is in the hiscore, and the topic says who won.
    let actions = quiz.message("zorg", "!hiscore", 8).expect("hiscore");
    assert_eq!(kinds(&actions), [Kind::Score]);
    assert_eq!(texts(&actions)[0], "1. **barbara**: 1 points");
    let topic = quiz.reset();
    assert!(texts(&topic)[0].contains("barbara"));
    assert!(texts(&topic)[0].contains("previous winner"));
}

#[test]
fn a_wrong_answer_is_ignored_until_the_bot_gives_up() {
    let mut quiz = game_with(long_bank());
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    assert!(
        quiz.message("barbara", "keine ahnung", 1)
            .expect("answer")
            .is_empty()
    );

    let actions = quiz.tick(20);
    assert_eq!(kinds(&actions), [Kind::Hint]);
    assert!(texts(&actions)[0].contains('*'), "{}", texts(&actions)[0]);

    // Giving up after five hints shows the answer.
    for second in 2..=5 {
        let actions = quiz.tick(second * 20);
        assert_eq!(kinds(&actions), [Kind::Hint], "hint {second}");
    }
    let actions = quiz.tick(120);
    assert_eq!(kinds(&actions), [Kind::Warning, Kind::Info]);
    assert!(
        texts(&actions)[0].contains("Schlafmohn"),
        "{}",
        texts(&actions)[0]
    );
    assert!(quiz.pending().is_none());
}

#[test]
fn a_hint_keeps_the_gaps_between_the_words() {
    let mut quiz = game_with(
        Bank::parse(
            r#"
[[question]]
category = "Orte"
question = "Welche Stadt ist der Big Apple?"
answer = "new york"
"#,
        )
        .expect("bank"),
    );
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);

    let hint = texts(&quiz.tick(20))[0].clone();
    let shown = hint
        .split_once('`')
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(shown, _)| shown.to_owned())
        .expect("the hinted answer in backticks");
    assert_eq!(shown.chars().count(), "new york".chars().count(), "{shown}");
    assert!(shown.contains(' '), "the gap stays visible: {shown}");
    // One letter per hint, the rest still hidden and the gap never a `*`.
    assert_eq!(shown.matches('*').count(), "newyork".len() - 1, "{shown}");
}

#[test]
fn a_one_letter_answer_hints_once_and_then_gives_up() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    let actions = quiz.tick(20);
    assert_eq!(kinds(&actions), [Kind::Hint]);
    assert!(texts(&actions)[0].contains("one character"));
    let actions = quiz.tick(40);
    assert_eq!(kinds(&actions), [Kind::Warning, Kind::Info]);
}

#[test]
fn hunger_stops_the_game_until_the_bot_is_fed() {
    let mut quiz = game_with(long_bank());
    quiz.add("barbara");
    quiz.add("zorg");
    let mut second = 0;
    for round in 0..3 {
        let actions = quiz.tick(second);
        assert_eq!(kinds(&actions), [Kind::Question], "round {round}");
        // Nobody answers: the hints and then the give-up close the question.
        while quiz.pending().is_some() {
            second += 20;
            quiz.tick(second);
        }
        second += rules().answer_gap_secs;
    }
    assert_eq!(quiz.hunger(), 3);
    // The next question is one past the stamina of three, so the bot complains once.
    let actions = quiz.tick(second);
    assert_eq!(kinds(&actions), [Kind::Warning]);
    assert!(texts(&actions)[0].contains("hungry"));
    assert!(quiz.tick(second + 1).is_empty());
    quiz.message("barbara", "!botsnack", second + 2)
        .expect("feed");
    let actions = quiz.tick(second + 3);
    assert_eq!(kinds(&actions), [Kind::Question]);
}

#[test]
fn an_answer_costs_one_hunger() {
    let mut quiz = game_with(long_bank());
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    assert_eq!(quiz.hunger(), 1);
    answer(&mut quiz, "barbara", 1);
    assert_eq!(quiz.hunger(), 0);
}

#[test]
fn stop_pauses_the_quiz_and_start_resumes_it() {
    let mut quiz = game_with(long_bank());
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    assert!(quiz.pending().is_some());

    // A player who is not a master cannot stop the game.
    let actions = quiz.message("barbara", "!stop", 1).expect("stop");
    assert!(
        texts(&actions)
            .iter()
            .any(|text| text.contains("not on my watch"))
    );

    let actions = quiz.message("Mumquiz", "!stop", 2).expect("stop");
    assert!(
        texts(&actions)[0].contains("quiz stopped"),
        "{}",
        texts(&actions)[0]
    );
    // The question that was open before the stop is dropped: nobody wins it later.
    assert!(quiz.pending().is_none());
    assert!(quiz.tick(3).is_empty());
    assert!(
        quiz.message("barbara", "Schlafmohn", 4)
            .expect("answer")
            .is_empty()
    );

    let actions = quiz.message("Mumquiz", "!start", 5).expect("start");
    assert!(
        texts(&actions)[0].contains("quiz started"),
        "{}",
        texts(&actions)[0]
    );
    let actions = quiz.tick(6);
    assert_eq!(kinds(&actions), [Kind::Question]);
    answer(&mut quiz, "barbara", 7);
    assert_eq!(points(&quiz, "barbara"), Some(1));
}

#[test]
fn help_reaches_players_privately_only() {
    let mut quiz = game();
    quiz.add("barbara");
    assert!(
        quiz.message("visitor", "!help", 0)
            .expect("help")
            .is_empty()
    );
    let actions = quiz.message("barbara", "!help", 0).expect("help");
    // One card with the lines, not one card per line.
    assert_eq!(actions.len(), 1, "help must be a single message");
    let Action::Private { quizzer, line } = &actions[0] else {
        panic!("expected a private line, got {:?}", actions[0]);
    };
    assert_eq!(quizzer, "barbara");
    assert_eq!(line.kind, Kind::Info);
    assert!(
        line.text.contains("**Quiz** is a quiz channel"),
        "{}",
        line.text
    );
    // channel info + bot info + the help lines.
    assert_eq!(line.text.matches('\n').count(), 7, "{}", line.text);
    assert!(line.text.contains("`!botsnack`"), "{}", line.text);
}

#[test]
fn master_commands_are_for_masters_only() {
    let mut quiz = game();
    quiz.add("barbara");
    let actions = quiz.message("barbara", "!reload", 0).expect("reload");
    assert_eq!(kinds(&actions), [Kind::Warning]);
    assert!(texts(&actions)[0].contains("not on my watch"));

    // The bot itself is a master, as upstream configured it.
    let actions = quiz.message("Mumquiz", "!join Lobby", 0).expect("join");
    assert!(actions.contains(&Action::JoinChannel("Lobby".to_owned())));

    // A master can make another player one.
    quiz.add("barbara");
    let actions = quiz.message("Mumquiz", "!op barbara", 0).expect("op");
    assert!(
        texts(&actions)
            .iter()
            .any(|text| text.contains("is a master"))
    );
    let actions = quiz.message("barbara", "!reload", 0).expect("reload");
    assert_eq!(kinds(&actions), [Kind::Info, Kind::Info]);
    assert!(texts(&actions)[1].contains("reloaded"));
    assert!(quiz.message("zorg", "!help", 0).expect("help").is_empty());
    // A master can take the role away again; the bot always keeps its own.
    let actions = quiz.message("Mumquiz", "!deop barbara", 0).expect("deop");
    assert!(
        texts(&actions)
            .iter()
            .any(|text| text.contains("no longer a master"))
    );
    let actions = quiz.message("barbara", "!join Lobby", 0).expect("join");
    assert!(
        texts(&actions)
            .iter()
            .any(|text| text.contains("not on my watch"))
    );
    let actions = quiz.message("Mumquiz", "!deop Mumquiz", 0).expect("deop");
    assert!(
        texts(&actions)
            .iter()
            .any(|text| text.contains("not on my watch"))
    );
}

#[test]
fn an_unknown_command_answers_like_the_upstream_bot() {
    let mut quiz = game();
    quiz.add("barbara");
    let actions = quiz.message("barbara", "!wat", 0).expect("command");
    assert_eq!(texts(&actions), ["... wat."]);
}

#[test]
fn score_lists_only_players_that_answered() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    answer(&mut quiz, "barbara", 1);
    let actions = quiz.message("zorg", "!score", 2).expect("score");
    assert_eq!(kinds(&actions), [Kind::Score]);
    assert_eq!(texts(&actions), ["1. **barbara**: 1 points"]);
    // An empty board answers with a line of its own.
    let mut quiet = game();
    quiet.add("barbara");
    quiet.add("zorg");
    let actions = quiet.message("zorg", "!score", 1).expect("score");
    assert_eq!(texts(&actions), ["no points yet this round."]);
    let actions = quiet.message("zorg", "!hiscore", 2).expect("hiscore");
    assert_eq!(texts(&actions), ["no wins yet, the board is empty."]);
}

#[test]
fn the_board_is_one_message_with_one_player_per_line() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    // Two correct answers in the same round, nobody at the target score of two yet.
    quiz.tick(0);
    answer(&mut quiz, "barbara", 1);
    quiz.tick(6);
    answer(&mut quiz, "zorg", 7);

    let actions = quiz.message("barbara", "!score", 8).expect("score");
    assert_eq!(kinds(&actions), [Kind::Score]);
    let board = texts(&actions)[0].clone();
    assert_eq!(board.matches('\n').count(), 1, "{board}");
    assert!(board.starts_with("1. **barbara**: 1 points"), "{board}");
    assert!(board.ends_with("1. **zorg**: 1 points"), "{board}");

    // One win, so the all-time board is a single line.
    quiz.tick(12);
    answer(&mut quiz, "barbara", 13);
    let actions = quiz.message("zorg", "!hiscore", 14).expect("hiscore");
    assert_eq!(kinds(&actions), [Kind::Score]);
    assert_eq!(texts(&actions), ["1. **barbara**: 1 points"]);
}

#[test]
fn a_rename_moves_the_points_and_the_hiscore() {
    let mut quiz = game();
    quiz.add("barbara");
    quiz.add("zorg");
    quiz.tick(0);
    answer(&mut quiz, "barbara", 1);
    let actions = quiz.rename("barbara", "hans").expect("rename");
    assert_eq!(points(&quiz, "hans"), Some(1));
    // No win yet, so nothing was kept in the hiscore.
    assert!(actions.is_empty());
}

#[test]
fn a_renamed_winner_keeps_the_hiscore_entry() {
    let mut quiz = game_with(long_bank());
    quiz.add("barbara");
    quiz.add("zorg");
    // Two correct answers reach the target score of two, so barbara wins once.
    quiz.tick(0);
    answer(&mut quiz, "barbara", 1);
    quiz.tick(6);
    answer(&mut quiz, "barbara", 7);
    let actions = quiz.rename("barbara", "hans").expect("rename");
    assert_eq!(texts(&actions), ["barbara is now hans, score kept."]);
}

#[test]
fn a_small_bank_must_reuse_its_questions() {
    let bank = Bank::parse(
        r#"
[[question]]
category = "A"
question = "erste"
answer = "eins"

[[question]]
category = "B"
question = "zweite"
answer = "zwei"
"#,
    )
    .expect("bank");
    // Two questions, 50 percent recent: one is always out of the bag, so they must alternate.
    let mut quiz = game_with(bank);
    quiz.add("barbara");
    quiz.add("zorg");
    let mut asked = Vec::new();
    let mut second = 0;
    while asked.len() < 5 {
        quiz.tick(second);
        if let Some(question) = quiz.pending() {
            asked.push(question.text.clone());
            answer(&mut quiz, "barbara", second);
            second += rules().answer_gap_secs;
        }
        second += 1;
    }
    assert_eq!(asked, vec!["erste", "zweite", "erste", "zweite", "erste"]);
}
