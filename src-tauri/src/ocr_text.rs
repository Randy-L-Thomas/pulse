//! Turn WhatsApp OCR dumps into one chat line per message.

const CHROME: &[&str] = &[
    "yesterday",
    "today",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "sticker",
    "unread",
    "chats",
    "online",
    "forwarded",
];

const CHROME_PHRASES: &[&str] = &[
    "type a message",
    "search or start a new chat",
    "search or start",
    "last seen",
    "today at",
    "tap to",
];

const SHORT_OK: &[&str] = &[
    "ok", "si", "sí", "no", "hola", "gracias", "jaja", "ja", "yes", "lol", "bye", "okis",
];

pub fn looks_like_wa_ocr(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    count_sub(&lower, "yesterday") >= 2
        || count_sub(&lower, "today") >= 2
        || (count_time_tokens(raw) >= 2 && chrome_word_hits(raw) >= 1)
}

pub fn format_wa_ocr(raw: &str) -> String {
    let stripped = strip_phrases(raw);
    let mut msgs = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let flush = |cur: &mut Vec<&str>, msgs: &mut Vec<String>| {
        if cur.is_empty() {
            return;
        }
        let s = cur.join(" ");
        cur.clear();
        if let Some(keep) = keep_message(&s) {
            msgs.push(keep);
        }
    };
    for tok in stripped.split_whitespace() {
        if is_separator(tok) {
            flush(&mut cur, &mut msgs);
        } else if is_message_start(tok)
            && !cur.is_empty()
            && keep_message(&cur.join(" ")).is_none()
        {
            flush(&mut cur, &mut msgs);
            cur.push(tok);
        } else {
            cur.push(tok);
        }
    }
    flush(&mut cur, &mut msgs);
    msgs.join("\n\n")
}

fn strip_phrases(raw: &str) -> String {
    let mut out = raw.to_string();
    for phrase in CHROME_PHRASES {
        loop {
            let lower = out.to_ascii_lowercase();
            if let Some(i) = lower.find(phrase) {
                out.replace_range(i..i + phrase.len(), " ");
            } else {
                break;
            }
        }
    }
    out
}

fn is_separator(tok: &str) -> bool {
    is_chrome_word(tok) || is_time_token(tok)
}

fn is_message_start(tok: &str) -> bool {
    let t = bare(tok);
    let t = t.trim_matches(|c: char| c == ':' || c == '.');
    matches!(
        t,
        "hola" | "buenas" | "buen" | "hello" | "hey" | "hi" | "good" | "gracias"
    )
}

fn bare(tok: &str) -> String {
    tok.chars()
        .filter(|c| c.is_alphanumeric() || *c == ':' || *c == '.')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_chrome_word(tok: &str) -> bool {
    let t = bare(tok);
    let t = t.trim_matches(|c: char| c == ':' || c == '.');
    CHROME.contains(&t)
}

fn is_time_token(tok: &str) -> bool {
    let t = bare(tok);
    if let Some((h, m)) = t.split_once(':').or_else(|| t.split_once('.')) {
        return parse_hour_min(h, m);
    }
    if t.len() == 4 && t.bytes().all(|b| b.is_ascii_digit()) {
        return parse_hour_min(&t[..2], &t[2..]);
    }
    false
}

fn parse_hour_min(h: &str, m: &str) -> bool {
    let Ok(hh) = h.parse::<u32>() else {
        return false;
    };
    let Ok(mm) = m.parse::<u32>() else {
        return false;
    };
    hh <= 23 && mm <= 59 && (h.len() <= 2) && m.len() == 2
}

fn keep_message(s: &str) -> Option<String> {
    let cleaned = s
        .trim()
        .trim_matches(|c: char| matches!(c, '\'' | '"' | '`' | ',' | '.' | ';'))
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return None;
    }
    let letters = cleaned.chars().filter(|c| c.is_alphabetic()).count();
    if letters == 0 {
        return None;
    }
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let simple: String = cleaned
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase();
    if SHORT_OK.contains(&simple.as_str()) {
        return Some(cleaned);
    }
    if words.len() >= 3 && letters >= 8 {
        return Some(cleaned);
    }
    if letters >= 18 {
        return Some(cleaned);
    }
    None
}

fn count_sub(hay: &str, needle: &str) -> usize {
    hay.match_indices(needle).count()
}

fn count_time_tokens(raw: &str) -> usize {
    raw.split_whitespace().filter(|t| is_time_token(t)).count()
}

fn chrome_word_hits(raw: &str) -> usize {
    raw.split_whitespace().filter(|t| is_chrome_word(t)).count()
}

#[cfg(test)]
mod tests {
    use super::{format_wa_ocr, looks_like_wa_ocr};

    #[test]
    fn parses_sidebar_jumble_into_two_chat_lines() {
        let raw = "i Yesterday Yesterday Yesterday Yesterday ntregado., Yesterday Yesterday io Gui,. O - Yesterday CISTE., Buenas tardes tiene paquete 1528 1525 hola buenas tardes y gracias' 1525";
        assert!(looks_like_wa_ocr(raw));
        let out = format_wa_ocr(raw);
        assert_eq!(
            out,
            "Buenas tardes tiene paquete\n\nhola buenas tardes y gracias"
        );
    }

    #[test]
    fn keeps_line_broken_messages_and_drops_clocks() {
        let raw = "Yesterday\nBuenas tardes tiene paquete\n15:25\nhola buenas tardes y gracias!\n15:25";
        assert_eq!(
            format_wa_ocr(raw),
            "Buenas tardes tiene paquete\n\nhola buenas tardes y gracias!"
        );
    }

    #[test]
    fn a_real_yesterday_sentence_is_not_wa_ocr() {
        assert!(!looks_like_wa_ocr(
            "Yesterday I picked up the package at the office."
        ));
    }

    #[test]
    fn keeps_short_chat_acks() {
        assert_eq!(format_wa_ocr("ok 15:25"), "ok");
        assert_eq!(format_wa_ocr("gracias"), "gracias");
    }
}
