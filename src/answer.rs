//! Answer matching. A player's message counts when it contains the answer, so "it is pluto" wins.
//! Compared case-insensitive and spellings that are the same word are treated as equal: `ä`/`ae`,
//! `ö`/`oe`, `ü`/`ue`, `ß`/`ss`, accents to their base letter, and punctuation ignored. A very
//! short answer must appear as its own word, otherwise "rot" would match "proton".

/// Shortest answer that still needs a whole word, not just a part of one.
const SHORT_ANSWER: usize = 3;

pub fn normalize(text: &str) -> String {
    let mapped: String = text
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | ' ' => c,
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ñ' => 'n',
            'ç' => 'c',
            'ß' => 's',
            _ => ' ',
        })
        .collect();
    // The same sound written two ways: "koerona" must read as "korona".
    let out = mapped
        .replace("ae", "a")
        .replace("oe", "o")
        .replace("ue", "u")
        .replace("ss", "s");
    out.split(' ')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the message contains one of the answers.
pub fn matches(answers: &[String], message: &str) -> bool {
    let message = normalize(message);
    answers.iter().any(|answer| {
        let answer = normalize(answer);
        if answer.is_empty() {
            return false;
        }
        if answer.len() <= SHORT_ANSWER {
            message.split(' ').any(|word| word == answer)
        } else {
            message.contains(&answer)
        }
    })
}
