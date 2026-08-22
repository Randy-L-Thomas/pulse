//! Instant Mexican Spanish ↔ American English for chat. Phrase-first, then words.

fn es_en_phrases() -> &'static [(&'static str, &'static str)] {
    &[
        ("buenas tardes tiene paquete", "Good afternoon, you have a package"),
        ("buenos dias tiene paquete", "Good morning, you have a package"),
        ("tiene un paquete", "you have a package"),
        ("tiene paquete", "you have a package"),
        ("buenas tardes", "Good afternoon"),
        ("buenos dias", "Good morning"),
        ("buenas noches", "Good evening"),
        ("hola como estas", "hello, how are you"),
        ("como estas", "how are you"),
        ("como esta", "how are you"),
        ("mucho gusto", "nice to meet you"),
        ("de nada", "you're welcome"),
        ("por favor", "please"),
        ("lo siento", "I'm sorry"),
        ("hasta luego", "see you later"),
        ("nos vemos", "see you"),
        ("esta bien", "that's fine"),
        ("está bien", "that's fine"),
        ("todo bien", "all good"),
        ("estoy bien", "I'm fine"),
        ("en camino", "on the way"),
        ("ya esta", "it's done"),
        ("ya está", "it's done"),
        ("te mando", "I'll send you"),
        ("te llamo", "I'll call you"),
        ("a que hora", "what time"),
        ("dónde esta", "where is"),
        ("donde esta", "where is"),
        ("cuanto cuesta", "how much is it"),
        ("no hay", "there is no"),
        ("si senor", "yes sir"),
        ("sí señor", "yes sir"),
        ("con permiso", "excuse me"),
        ("muy amable", "very kind"),
        ("que tal", "how's it going"),
        ("qué tal", "how's it going"),
    ]
}

fn en_es_phrases() -> &'static [(&'static str, &'static str)] {
    &[
        ("good afternoon, you have a package", "Buenas tardes, tiene paquete"),
        ("you have a package", "tiene paquete"),
        ("good afternoon", "Buenas tardes"),
        ("good morning", "Buenos días"),
        ("good evening", "Buenas noches"),
        ("good night", "Buenas noches"),
        ("hello, how are you", "hola, ¿cómo estás?"),
        ("how are you", "¿cómo estás?"),
        ("nice to meet you", "mucho gusto"),
        ("you're welcome", "de nada"),
        ("thank you", "gracias"),
        ("thanks", "gracias"),
        ("please", "por favor"),
        ("i'm sorry", "lo siento"),
        ("see you later", "hasta luego"),
        ("see you", "nos vemos"),
        ("that's fine", "está bien"),
        ("all good", "todo bien"),
        ("i'm fine", "estoy bien"),
        ("on the way", "en camino"),
        ("what time", "¿a qué hora?"),
        ("how much", "¿cuánto?"),
        ("excuse me", "con permiso"),
    ]
}

fn es_en_words() -> &'static [(&'static str, &'static str)] {
    &[
        ("hola", "hello"),
        ("gracias", "thank you"),
        ("si", "yes"),
        ("sí", "yes"),
        ("no", "no"),
        ("ok", "ok"),
        ("okis", "ok"),
        ("vale", "ok"),
        ("jaja", "haha"),
        ("ja", "ha"),
        ("paquete", "package"),
        ("paquetes", "packages"),
        ("tiene", "has"),
        ("tengo", "I have"),
        ("tenemos", "we have"),
        ("mensaje", "message"),
        ("foto", "photo"),
        ("fotos", "photos"),
        ("llamada", "call"),
        ("ahora", "now"),
        ("luego", "later"),
        ("hoy", "today"),
        ("ayer", "yesterday"),
        ("manana", "tomorrow"),
        ("mañana", "tomorrow"),
        ("aqui", "here"),
        ("aquí", "here"),
        ("alli", "there"),
        ("allí", "there"),
        ("bien", "good"),
        ("mal", "bad"),
        ("espera", "wait"),
        ("listo", "ready"),
        ("perfecto", "perfect"),
        ("claro", "sure"),
        ("bueno", "well"),
        ("tambien", "also"),
        ("también", "also"),
        ("porque", "because"),
        ("pero", "but"),
        ("con", "with"),
        ("sin", "without"),
        ("para", "for"),
        ("por", "for"),
        ("de", "of"),
        ("el", "the"),
        ("la", "the"),
        ("los", "the"),
        ("las", "the"),
        ("un", "a"),
        ("una", "a"),
        ("y", "and"),
        ("o", "or"),
        ("que", "that"),
        ("qué", "what"),
        ("en", "in"),
        ("a", "to"),
        ("es", "is"),
        ("esta", "is"),
        ("está", "is"),
        ("son", "are"),
        ("hay", "there is"),
        ("puedo", "I can"),
        ("puede", "can"),
        ("quiero", "I want"),
        ("necesito", "I need"),
        ("mando", "I send"),
        ("envio", "I send"),
        ("envío", "I send"),
        ("recibi", "I received"),
        ("recibí", "I received"),
        ("entregado", "delivered"),
        ("oficina", "office"),
        ("casa", "house"),
        ("calle", "street"),
        ("numero", "number"),
        ("número", "number"),
        ("yo", "I"),
        ("tu", "you"),
        ("tú", "you"),
        ("usted", "you"),
        ("el", "he"),
        ("él", "he"),
        ("ella", "she"),
        ("como", "how"),
        ("cómo", "how"),
        ("donde", "where"),
        ("dónde", "where"),
        ("cuando", "when"),
        ("cuándo", "when"),
        ("quien", "who"),
        ("quién", "who"),
        ("tardes", "afternoon"),
        ("dias", "days"),
        ("días", "days"),
        ("noches", "nights"),
        ("buenas", "good"),
        ("buenos", "good"),
        ("senor", "sir"),
        ("señor", "sir"),
        ("porfa", "please"),
        ("favor", "favor"),
    ]
}

fn en_es_words() -> &'static [(&'static str, &'static str)] {
    &[
        ("hello", "hola"),
        ("hi", "hola"),
        ("hey", "hola"),
        ("yes", "sí"),
        ("no", "no"),
        ("ok", "ok"),
        ("okay", "ok"),
        ("thanks", "gracias"),
        ("package", "paquete"),
        ("packages", "paquetes"),
        ("has", "tiene"),
        ("have", "tiene"),
        ("message", "mensaje"),
        ("photo", "foto"),
        ("call", "llamada"),
        ("now", "ahora"),
        ("later", "luego"),
        ("today", "hoy"),
        ("yesterday", "ayer"),
        ("tomorrow", "mañana"),
        ("here", "aquí"),
        ("there", "allí"),
        ("good", "bien"),
        ("bad", "mal"),
        ("wait", "espera"),
        ("ready", "listo"),
        ("perfect", "perfecto"),
        ("sure", "claro"),
        ("well", "bueno"),
        ("also", "también"),
        ("because", "porque"),
        ("but", "pero"),
        ("with", "con"),
        ("without", "sin"),
        ("for", "para"),
        ("the", "el"),
        ("a", "un"),
        ("an", "un"),
        ("and", "y"),
        ("or", "o"),
        ("that", "que"),
        ("what", "qué"),
        ("in", "en"),
        ("to", "a"),
        ("is", "es"),
        ("are", "son"),
        ("i", "yo"),
        ("you", "tú"),
        ("he", "él"),
        ("she", "ella"),
        ("how", "cómo"),
        ("where", "dónde"),
        ("when", "cuándo"),
        ("who", "quién"),
        ("please", "por favor"),
        ("afternoon", "tardes"),
        ("morning", "días"),
        ("sir", "señor"),
        ("office", "oficina"),
        ("house", "casa"),
        ("street", "calle"),
        ("number", "número"),
        ("delivered", "entregado"),
    ]
}

fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            'Á' | 'À' => 'a',
            'É' | 'È' => 'e',
            'Í' | 'Ì' => 'i',
            'Ó' | 'Ò' => 'o',
            'Ú' | 'Ù' | 'Ü' => 'u',
            'Ñ' => 'n',
            '¿' | '¡' => ' ',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn split_speaker(line: &str) -> (Option<String>, &str) {
    let Some((who, rest)) = line.split_once(':') else {
        return (None, line);
    };
    let who = who.trim();
    if who.is_empty() || who.len() > 24 || who.chars().any(|c| c == ',' || c == '?' || c == '!') {
        return (None, line);
    }
    let words = who.split_whitespace().count();
    if words == 0 || words > 3 {
        return (None, line);
    }
    (Some(who.to_string()), rest.trim())
}

fn lookup<'a>(table: &'a [(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    let k = fold(key);
    table
        .iter()
        .find(|(src, _)| fold(src) == k)
        .map(|(_, dst)| *dst)
}

fn translate_body(from: &str, to: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return String::new();
    }
    let phrases = if from == "es" && to == "en" {
        es_en_phrases()
    } else {
        en_es_phrases()
    };
    let words = if from == "es" && to == "en" {
        es_en_words()
    } else {
        en_es_words()
    };
    if let Some(hit) = lookup(phrases, body) {
        return hit.to_string();
    }
    let toks: Vec<&str> = body.split_whitespace().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < toks.len() {
        let mut hit = None;
        let max = 6.min(toks.len() - i);
        for n in (1..=max).rev() {
            let chunk = toks[i..i + n].join(" ");
            if let Some(t) = lookup(phrases, &chunk) {
                hit = Some((n, t.to_string()));
                break;
            }
        }
        if let Some((n, t)) = hit {
            out.push(t);
            i += n;
            continue;
        }
        let (core, trail) = peel(toks[i]);
        if let Some(t) = lookup(words, &core) {
            out.push(format!("{t}{trail}"));
        } else {
            out.push(toks[i].to_string());
        }
        i += 1;
    }
    let s = out.join(" ");
    cap_first(&s)
}

fn peel(tok: &str) -> (String, String) {
    let mut end = tok.len();
    for (i, c) in tok.char_indices().rev() {
        if c.is_alphanumeric() || c == 'á' || c == 'é' || c == 'í' || c == 'ó' || c == 'ú' || c == 'ñ' || c == 'ü'
        {
            break;
        }
        end = i;
    }
    (tok[..end].to_string(), tok[end..].to_string())
}

fn cap_first(s: &str) -> String {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.extend(chars);
    out
}

pub fn translate_lex(from: &str, to: &str, src: &str) -> String {
    let mut lines = Vec::new();
    for line in src.split('\n') {
        if line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let (who, body) = split_speaker(line);
        let t = translate_body(from, to, body);
        if let Some(who) = who {
            lines.push(if t.is_empty() {
                format!("{who}:")
            } else {
                format!("{who}: {t}")
            });
        } else {
            lines.push(t);
        }
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::translate_lex;

    #[test]
    fn phrases_and_speaker_labels() {
        assert_eq!(
            translate_lex("es", "en", "Buenas tardes tiene paquete"),
            "Good afternoon, you have a package"
        );
        assert_eq!(
            translate_lex("es", "en", "Me: hola\n\nThem: gracias"),
            "Me: Hello\n\nThem: Thank you"
        );
        assert_eq!(
            translate_lex("es", "en", "Maria: buenas tardes"),
            "Maria: Good afternoon"
        );
    }

    #[test]
    fn unknown_tokens_stay() {
        let out = translate_lex("es", "en", "CAM-MCP 22AUG26");
        assert!(out.contains("CAM-MCP"));
        assert!(out.contains("22AUG26"));
    }

    #[test]
    fn english_back_to_spanish() {
        assert_eq!(translate_lex("en", "es", "thank you"), "gracias");
        assert_eq!(
            translate_lex("en", "es", "Good afternoon"),
            "Buenas tardes"
        );
    }
}
