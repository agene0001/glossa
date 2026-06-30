//! `glossa-lemma` — resolve inflected surface forms back to their lexeme.
//!
//! V1 vocabulary is stored flat (one entry per lemma), but real text contains
//! conjugated verbs and plurals — `comí`/`comería` for `comer`, `mange` for
//! `manger`, `gatos` for `gato`. Without this, those show as "unknown" even
//! when the learner knows the base word, which corrupts both highlighting and
//! the mastery graph.
//!
//! Because the inventory is a **closed list**, we don't need a general
//! lemmatizer — only complete inflection of the lemmas we actually have. So we
//! generate the surface forms for each seeded lexeme (regular Spanish/French
//! conjugation across the common tenses + plural rules, plus a curated table
//! for high-frequency irregulars) and build a `surface form → LexemeId` index.
//! Pure, no external data, fully offline; a missing form just falls back to
//! "unknown" (same as before), so this is strictly an improvement.

use std::collections::HashMap;

use glossa_core::{Lexeme, LexemeId, PartOfSpeech};

/// Build a `lowercased surface form → LexemeId` index from the inventory.
///
/// Exact lemmas are authoritative (inserted last); generated forms fill in
/// around them and never overwrite a real lemma or an earlier generated form.
pub fn build_form_index(lexemes: &[Lexeme]) -> HashMap<String, LexemeId> {
    let mut map: HashMap<String, LexemeId> = HashMap::new();
    for lex in lexemes {
        for form in surface_forms(lex) {
            map.entry(form).or_insert(lex.id);
        }
    }
    for lex in lexemes {
        map.insert(lex.lemma.to_lowercase(), lex.id);
    }
    map
}

/// One labeled cell of a present-tense conjugation, e.g. `yo` (I) → `soy`.
/// Used to *teach* the link between an infinitive and the forms a learner meets
/// in text (so `ser` and `soy` aren't two unrelated mysteries).
#[derive(Debug, Clone, PartialEq)]
pub struct Conjugation {
    /// Subject pronoun in the target language ("yo", "je").
    pub pronoun: &'static str,
    /// Its native-language gloss ("I", "we").
    pub gloss: &'static str,
    /// The conjugated verb form for this person ("soy", "suis").
    pub form: String,
    /// True when this form deviates from what the regular rule would produce —
    /// i.e. it's irregular and worth flagging so the learner doesn't guess it
    /// from the pattern (e.g. `essen → er isst`, not `esst`).
    pub irregular: bool,
}

/// Spanish subject pronouns, in the order present-tense tables are emitted.
const ES_PRONOUNS: [(&str, &str); 5] = [
    ("yo", "I"),
    ("tú", "you"),
    ("él/ella", "he/she"),
    ("nosotros", "we"),
    ("ellos/ellas", "they"),
];

/// French subject pronouns, same ordering.
const FR_PRONOUNS: [(&str, &str); 5] = [
    ("je", "I"),
    ("tu", "you"),
    ("il/elle", "he/she"),
    ("nous", "we"),
    ("ils/elles", "they"),
];

/// German subject pronouns, same ordering.
const DE_PRONOUNS: [(&str, &str); 5] = [
    ("ich", "I"),
    ("du", "you"),
    ("er/sie/es", "he/she/it"),
    ("wir", "we"),
    ("sie", "they"),
];

/// The present-tense conjugation of a verb lexeme, one row per pronoun. Empty
/// for non-verbs or languages/verbs we don't model — callers just skip it.
pub fn present_tense(lex: &Lexeme) -> Vec<Conjugation> {
    if lex.pos != PartOfSpeech::Verb {
        return Vec::new();
    }
    let lemma = lex.lemma.to_lowercase();
    // `actual` is the real conjugation; `regular` is what the plain rule would
    // give, so we can flag each cell that deviates (i.e. is irregular).
    let (actual, regular, pronouns) = match lex.language.as_str() {
        "es" => (spanish_present(&lemma), spanish_present_regular(&lemma), &ES_PRONOUNS),
        "fr" => (french_present(&lemma), french_present_regular(&lemma), &FR_PRONOUNS),
        "de" => (german_present(&lemma), german_present_regular(&lemma), &DE_PRONOUNS),
        _ => return Vec::new(),
    };
    let Some(actual) = actual else {
        return Vec::new();
    };
    let flags: [bool; 5] = match &regular {
        Some(r) => std::array::from_fn(|i| actual[i] != r[i]),
        None => [false; 5],
    };
    pronouns
        .iter()
        .zip(actual)
        .enumerate()
        .map(|(i, (&(pronoun, gloss), form))| Conjugation {
            pronoun,
            gloss,
            form,
            irregular: flags[i],
        })
        .collect()
}

/// Present-tense forms [yo, tú, él, nosotros, ellos] for a Spanish verb.
fn spanish_present(lemma: &str) -> Option<[String; 5]> {
    if let Some(forms) = spanish_present_irregular(lemma) {
        return Some(forms.map(String::from));
    }
    spanish_present_regular(lemma)
}

/// What the regular -ar/-er/-ir rule alone produces (the baseline for flagging
/// irregular forms).
fn spanish_present_regular(lemma: &str) -> Option<[String; 5]> {
    let (stem, ends) = if let Some(s) = lemma.strip_suffix("ar") {
        (s, ["o", "as", "a", "amos", "an"])
    } else if let Some(s) = lemma.strip_suffix("er") {
        (s, ["o", "es", "e", "emos", "en"])
    } else if let Some(s) = lemma.strip_suffix("ir") {
        (s, ["o", "es", "e", "imos", "en"])
    } else {
        return None;
    };
    Some(ends.map(|e| format!("{stem}{e}")))
}

/// Curated present tense for high-frequency irregular Spanish verbs.
fn spanish_present_irregular(lemma: &str) -> Option<[&'static str; 5]> {
    Some(match lemma {
        "ser" => ["soy", "eres", "es", "somos", "son"],
        "estar" => ["estoy", "estás", "está", "estamos", "están"],
        "ir" => ["voy", "vas", "va", "vamos", "van"],
        "haber" => ["he", "has", "ha", "hemos", "han"],
        "tener" => ["tengo", "tienes", "tiene", "tenemos", "tienen"],
        "hacer" => ["hago", "haces", "hace", "hacemos", "hacen"],
        "poder" => ["puedo", "puedes", "puede", "podemos", "pueden"],
        "querer" => ["quiero", "quieres", "quiere", "queremos", "quieren"],
        "decir" => ["digo", "dices", "dice", "decimos", "dicen"],
        "ver" => ["veo", "ves", "ve", "vemos", "ven"],
        "dar" => ["doy", "das", "da", "damos", "dan"],
        "saber" => ["sé", "sabes", "sabe", "sabemos", "saben"],
        "venir" => ["vengo", "vienes", "viene", "venimos", "vienen"],
        "poner" => ["pongo", "pones", "pone", "ponemos", "ponen"],
        "salir" => ["salgo", "sales", "sale", "salimos", "salen"],
        "pensar" => ["pienso", "piensas", "piensa", "pensamos", "piensan"],
        "volver" => ["vuelvo", "vuelves", "vuelve", "volvemos", "vuelven"],
        "encontrar" => ["encuentro", "encuentras", "encuentra", "encontramos", "encuentran"],
        "seguir" => ["sigo", "sigues", "sigue", "seguimos", "siguen"],
        "entender" => ["entiendo", "entiendes", "entiende", "entendemos", "entienden"],
        _ => return None,
    })
}

/// Present-tense forms [je, tu, il, nous, ils] for a French verb.
fn french_present(lemma: &str) -> Option<[String; 5]> {
    if let Some(forms) = french_present_irregular(lemma) {
        return Some(forms.map(String::from));
    }
    french_present_regular(lemma)
}

/// What the regular -er rule alone produces (baseline for flagging irregulars).
fn french_present_regular(lemma: &str) -> Option<[String; 5]> {
    let stem = lemma.strip_suffix("er")?;
    // nous keeps a soft g/c: manger → mangeons, commencer → commençons.
    let nous = if lemma.ends_with("ger") {
        format!("{stem}eons")
    } else if lemma.ends_with("cer") {
        format!("{}çons", &stem[..stem.len() - 1])
    } else {
        format!("{stem}ons")
    };
    Some([
        format!("{stem}e"),
        format!("{stem}es"),
        format!("{stem}e"),
        nous,
        format!("{stem}ent"),
    ])
}

/// Curated present tense for high-frequency irregular French verbs.
fn french_present_irregular(lemma: &str) -> Option<[&'static str; 5]> {
    Some(match lemma {
        "être" => ["suis", "es", "est", "sommes", "sont"],
        "avoir" => ["ai", "as", "a", "avons", "ont"],
        "aller" => ["vais", "vas", "va", "allons", "vont"],
        "faire" => ["fais", "fais", "fait", "faisons", "font"],
        "vouloir" => ["veux", "veux", "veut", "voulons", "veulent"],
        "pouvoir" => ["peux", "peux", "peut", "pouvons", "peuvent"],
        "boire" => ["bois", "bois", "boit", "buvons", "boivent"],
        "voir" => ["vois", "vois", "voit", "voyons", "voient"],
        "venir" => ["viens", "viens", "vient", "venons", "viennent"],
        "dire" => ["dis", "dis", "dit", "disons", "disent"],
        "lire" => ["lis", "lis", "lit", "lisons", "lisent"],
        "savoir" => ["sais", "sais", "sait", "savons", "savent"],
        "acheter" => ["achète", "achètes", "achète", "achetons", "achètent"],
        _ => return None,
    })
}

/// All surface forms we recognize for one lexeme (always includes the lemma).
pub fn surface_forms(lex: &Lexeme) -> Vec<String> {
    let lemma = lex.lemma.to_lowercase();
    let mut forms = vec![lemma.clone()];
    match lex.language.as_str() {
        "es" => spanish_forms(&lemma, lex.pos, &mut forms),
        "fr" => french_forms(&lemma, lex.pos, &mut forms),
        "de" => german_forms(&lemma, lex.pos, &mut forms),
        _ => {}
    }
    forms.retain(|f| !f.is_empty());
    forms
}

fn add(forms: &mut Vec<String>, stem: &str, suffixes: &[&str]) {
    for s in suffixes {
        forms.push(format!("{stem}{s}"));
    }
}

fn ends_with_vowel(s: &str) -> bool {
    matches!(
        s.chars().last(),
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú')
    )
}

// --- Spanish -------------------------------------------------------------

fn spanish_forms(lemma: &str, pos: PartOfSpeech, forms: &mut Vec<String>) {
    if pos == PartOfSpeech::Verb {
        if let Some(extra) = spanish_irregular(lemma) {
            forms.extend(extra.iter().map(|s| s.to_string()));
            return; // curated forms cover irregulars; skip regular generation
        }
        // future + conditional are formed on the full infinitive (hablar → hablaré, hablaría).
        let fut_cond = &["é", "ás", "á", "emos", "éis", "án", "ía", "ías", "íamos", "íais", "ían"];
        if let Some(stem) = lemma.strip_suffix("ar") {
            add(forms, stem, &[
                "o", "as", "a", "amos", "áis", "an", // present
                "é", "aste", "ó", "asteis", "aron", // preterite (amos shared with present)
                "aba", "abas", "ábamos", "abais", "aban", // imperfect
                "e", "es", "emos", "en", // present subjunctive
                "ando", "ado", "ada", "ados", "adas", // gerund / participle
            ]);
            add(forms, lemma, fut_cond);
            // -car / -gar / -zar spelling changes (busqué, llegué, empecé, + subjunctive).
            if let Some(b) = lemma.strip_suffix("car") {
                add(forms, b, &["qué", "que", "ques", "quemos", "quen"]);
            } else if let Some(b) = lemma.strip_suffix("gar") {
                add(forms, b, &["gué", "gue", "gues", "guemos", "guen"]);
            } else if let Some(b) = lemma.strip_suffix("zar") {
                add(forms, b, &["cé", "ce", "ces", "cemos", "cen"]);
            }
        } else if let Some(stem) = lemma.strip_suffix("er") {
            add(forms, stem, &[
                "o", "es", "e", "emos", "éis", "en", // present
                "í", "iste", "ió", "imos", "isteis", "ieron", // preterite
                "ía", "ías", "íamos", "íais", "ían", // imperfect
                "a", "as", "amos", "áis", "an", // present subjunctive
                "iendo", "ido", "ida", "idos", "idas", // gerund / participle
            ]);
            add(forms, lemma, fut_cond);
        } else if let Some(stem) = lemma.strip_suffix("ir") {
            add(forms, stem, &[
                "o", "es", "e", "imos", "ís", "en", // present
                "í", "iste", "ió", "imos", "isteis", "ieron", // preterite
                "ía", "ías", "íamos", "íais", "ían", // imperfect
                "a", "as", "amos", "áis", "an", // present subjunctive
                "iendo", "ido", "ida", "idos", "idas", // gerund / participle
            ]);
            add(forms, lemma, fut_cond);
        }
    } else if matches!(pos, PartOfSpeech::Noun | PartOfSpeech::Adjective) {
        if let Some(base) = lemma.strip_suffix('z') {
            forms.push(format!("{base}ces")); // vez → veces, feliz → felices
        } else if ends_with_vowel(lemma) {
            forms.push(format!("{lemma}s"));
        } else {
            forms.push(format!("{lemma}es"));
        }
        if pos == PartOfSpeech::Adjective {
            if let Some(base) = lemma.strip_suffix('o') {
                add(forms, base, &["a", "os", "as"]); // gender / number agreement
            }
        }
    }
}

/// Curated forms (present + preterite + common subjunctive/future/imperfect)
/// for high-frequency irregular Spanish verbs.
fn spanish_irregular(lemma: &str) -> Option<&'static [&'static str]> {
    let forms: &[&str] = match lemma {
        "ser" => &["soy", "eres", "es", "somos", "son", "fui", "fuiste", "fue", "fueron", "era", "eras", "éramos", "eran", "sea", "sean", "seré", "sería"],
        "estar" => &["estoy", "estás", "está", "estamos", "están", "estaba", "estaban", "estuve", "estuvo", "esté", "estén", "estaré"],
        "ir" => &["voy", "vas", "va", "vamos", "van", "fui", "fue", "fueron", "iba", "ibas", "íbamos", "iban", "vaya", "vayan", "iré", "iría"],
        "haber" => &["he", "has", "ha", "hemos", "han", "hay", "había", "habían", "haya", "habrá"],
        "tener" => &["tengo", "tienes", "tiene", "tenemos", "tienen", "tuve", "tuvo", "tenía", "tenían", "tenga", "tendré", "tendría"],
        "hacer" => &["hago", "haces", "hace", "hacemos", "hacen", "hice", "hizo", "hacía", "haga", "haré", "haría", "hecho"],
        "poder" => &["puedo", "puedes", "puede", "podemos", "pueden", "pude", "pudo", "podía", "pueda", "podré", "podría"],
        "querer" => &["quiero", "quieres", "quiere", "queremos", "quieren", "quise", "quiso", "quería", "quiera", "querré", "querría"],
        "decir" => &["digo", "dices", "dice", "decimos", "dicen", "dije", "dijo", "decía", "diga", "diré", "dicho"],
        "ver" => &["veo", "ves", "ve", "vemos", "ven", "vi", "vio", "veía", "veían", "vea", "veré", "visto"],
        "dar" => &["doy", "das", "da", "damos", "dan", "di", "dio", "daba", "dé", "daré"],
        "saber" => &["sé", "sabes", "sabe", "sabemos", "saben", "supe", "supo", "sabía", "sepa", "sabré"],
        "venir" => &["vengo", "vienes", "viene", "venimos", "vienen", "vine", "vino", "venía", "venga", "vendré"],
        "poner" => &["pongo", "pones", "pone", "ponemos", "ponen", "puse", "puso", "ponía", "ponga", "pondré", "puesto"],
        "salir" => &["salgo", "sales", "sale", "salimos", "salen", "salí", "salía", "salga", "saldré"],
        "pensar" => &["pienso", "piensas", "piensa", "pensamos", "piensan", "pensé", "pensaba", "piense"],
        "volver" => &["vuelvo", "vuelves", "vuelve", "volvemos", "vuelven", "volví", "volvía", "vuelva", "vuelto"],
        "encontrar" => &["encuentro", "encuentras", "encuentra", "encontramos", "encuentran", "encontré", "encuentre"],
        "seguir" => &["sigo", "sigues", "sigue", "seguimos", "siguen", "seguí", "siguió", "siga"],
        "entender" => &["entiendo", "entiendes", "entiende", "entendemos", "entienden", "entendí", "entendía", "entienda"],
        _ => return None,
    };
    Some(forms)
}

// --- French --------------------------------------------------------------

fn french_forms(lemma: &str, pos: PartOfSpeech, forms: &mut Vec<String>) {
    if pos == PartOfSpeech::Verb {
        if let Some(extra) = french_irregular(lemma) {
            forms.extend(extra.iter().map(|s| s.to_string()));
            return;
        }
        if let Some(stem) = lemma.strip_suffix("er") {
            add(forms, stem, &[
                "e", "es", "e", "ons", "ez", "ent", // present (+ pres. subjunctive overlap)
                "ais", "ait", "ions", "iez", "aient", // imperfect
                "é", "ée", "és", "ées", // past participle
                "ant", // present participle
            ]);
            // future + conditional on the infinitive (parler → parlerai, parlerais).
            add(forms, lemma, &["ai", "as", "a", "ons", "ez", "ont", "ais", "ait", "ions", "iez", "aient"]);
            // -ger/-cer keep a soft g/c before a/o: mangeons, mangeait; commençait.
            if let Some(b) = lemma.strip_suffix("r") {
                if b.ends_with("ge") {
                    add(forms, b, &["ons", "ais", "ait", "aient", "ant"]);
                }
            }
            if let Some(b) = lemma.strip_suffix("cer") {
                add(forms, &format!("{b}ç"), &["ons", "ais", "ait", "aient", "ant"]);
            }
        }
    } else if matches!(pos, PartOfSpeech::Noun | PartOfSpeech::Adjective) {
        if !matches!(lemma.chars().last(), Some('s' | 'x' | 'z')) {
            forms.push(format!("{lemma}s"));
        }
        if pos == PartOfSpeech::Adjective && !ends_with_vowel(lemma) {
            forms.push(format!("{lemma}e")); // rough feminine
            forms.push(format!("{lemma}es"));
        }
    }
}

/// Curated forms for high-frequency irregular French verbs.
fn french_irregular(lemma: &str) -> Option<&'static [&'static str]> {
    let forms: &[&str] = match lemma {
        "être" => &["suis", "es", "est", "sommes", "êtes", "sont", "étais", "était", "étaient", "été", "sera", "serait", "soit"],
        "avoir" => &["ai", "as", "a", "avons", "avez", "ont", "avais", "avait", "avaient", "eu", "aura", "aurait", "ait"],
        "aller" => &["vais", "vas", "va", "allons", "allez", "vont", "allais", "allait", "allé", "allée", "irai", "ira", "iront", "irait"],
        "faire" => &["fais", "fait", "faisons", "faites", "font", "faisais", "faisait", "fait", "fera", "ferait", "fasse"],
        "vouloir" => &["veux", "veut", "voulons", "voulez", "veulent", "voulais", "voulait", "voulu", "voudrais", "voudrait"],
        "pouvoir" => &["peux", "peut", "pouvons", "pouvez", "peuvent", "pouvais", "pouvait", "pu", "pourrait"],
        "boire" => &["bois", "boit", "buvons", "buvez", "boivent", "buvais", "bu"],
        "voir" => &["vois", "voit", "voyons", "voyez", "voient", "voyais", "vu", "verra"],
        "venir" => &["viens", "vient", "venons", "venez", "viennent", "venais", "venu", "viendra"],
        "dire" => &["dis", "dit", "disons", "dites", "disent", "disais", "dit"],
        "lire" => &["lis", "lit", "lisons", "lisez", "lisent", "lisais", "lu"],
        "savoir" => &["sais", "sait", "savons", "savez", "savent", "savais", "su", "saura"],
        _ => return None,
    };
    Some(forms)
}

// --- German --------------------------------------------------------------

/// The present-tense verb stem: drop the infinitive `-en` (or trailing `-n`).
fn german_stem(lemma: &str) -> &str {
    if let Some(s) = lemma.strip_suffix("en") {
        s
    } else if let Some(s) = lemma.strip_suffix('n') {
        s
    } else {
        lemma
    }
}

/// Present-tense forms [ich, du, er, wir, sie] for a German verb.
fn german_present(lemma: &str) -> Option<[String; 5]> {
    if let Some(forms) = german_present_irregular(lemma) {
        return Some(forms.map(String::from));
    }
    german_present_regular(lemma)
}

/// What the regular German present rule alone produces (baseline for flagging
/// irregulars).
fn german_present_regular(lemma: &str) -> Option<[String; 5]> {
    let stem = german_stem(lemma);
    // du/er pick up an extra -e after t/d (arbeitest); a sibilant stem collapses
    // the du -st to -t (du reist, not reisst).
    let du = if stem.ends_with(['s', 'ß', 'z', 'x']) {
        format!("{stem}t")
    } else if stem.ends_with(['t', 'd']) {
        format!("{stem}est")
    } else {
        format!("{stem}st")
    };
    let er = if stem.ends_with(['t', 'd']) {
        format!("{stem}et")
    } else {
        format!("{stem}t")
    };
    Some([format!("{stem}e"), du, er, lemma.to_string(), lemma.to_string()])
}

/// Curated present tense for high-frequency irregular German verbs (stem
/// changes and the modals).
fn german_present_irregular(lemma: &str) -> Option<[&'static str; 5]> {
    Some(match lemma {
        "sein" => ["bin", "bist", "ist", "sind", "sind"],
        "haben" => ["habe", "hast", "hat", "haben", "haben"],
        "werden" => ["werde", "wirst", "wird", "werden", "werden"],
        "können" => ["kann", "kannst", "kann", "können", "können"],
        "wollen" => ["will", "willst", "will", "wollen", "wollen"],
        "müssen" => ["muss", "musst", "muss", "müssen", "müssen"],
        "essen" => ["esse", "isst", "isst", "essen", "essen"],
        "sprechen" => ["spreche", "sprichst", "spricht", "sprechen", "sprechen"],
        "fahren" => ["fahre", "fährst", "fährt", "fahren", "fahren"],
        "helfen" => ["helfe", "hilfst", "hilft", "helfen", "helfen"],
        "sehen" => ["sehe", "siehst", "sieht", "sehen", "sehen"],
        "lesen" => ["lese", "liest", "liest", "lesen", "lesen"],
        "nehmen" => ["nehme", "nimmst", "nimmt", "nehmen", "nehmen"],
        "geben" => ["gebe", "gibst", "gibt", "geben", "geben"],
        _ => return None,
    })
}

/// Surface forms for one German lexeme. Verbs get their present tense (plus a
/// regular past participle); nouns/adjectives keep just the lemma — German
/// plurals and declensions are too irregular to generate reliably.
fn german_forms(lemma: &str, pos: PartOfSpeech, forms: &mut Vec<String>) {
    if pos == PartOfSpeech::Verb {
        if let Some(present) = german_present(lemma) {
            forms.extend(present);
        }
        // Regular weak past participle: ge- + stem + -t (gemacht, gelernt).
        if german_present_irregular(lemma).is_none() {
            let stem = german_stem(lemma);
            forms.push(format!("ge{stem}t"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_core::LanguageCode;

    fn lex(id: i64, lang: &str, lemma: &str, pos: PartOfSpeech) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::new(lang),
            lemma: lemma.into(),
            pos,
            frequency_rank: id as u32,
            gloss: None,
        }
    }

    #[test]
    fn spanish_tenses_resolve() {
        let lexemes = vec![
            lex(1, "es", "comer", PartOfSpeech::Verb),
            lex(2, "es", "ser", PartOfSpeech::Verb),
            lex(3, "es", "hablar", PartOfSpeech::Verb),
            lex(4, "es", "buscar", PartOfSpeech::Verb),
            lex(5, "es", "gato", PartOfSpeech::Noun),
            lex(6, "es", "vez", PartOfSpeech::Noun),
        ];
        let idx = build_form_index(&lexemes);
        let hit = |w: &str| idx.get(w).copied();
        // comer: present / preterite / imperfect / future / conditional / participle
        for w in ["como", "comió", "comía", "comeré", "comería", "comido"] {
            assert_eq!(hit(w), Some(LexemeId(1)), "{w}");
        }
        // ser irregular present + past
        for w in ["soy", "es", "fue", "era", "sería"] {
            assert_eq!(hit(w), Some(LexemeId(2)), "{w}");
        }
        // hablar future/imperfect/subjunctive
        for w in ["hablo", "hablé", "hablaba", "hablaré", "hable"] {
            assert_eq!(hit(w), Some(LexemeId(3)), "{w}");
        }
        assert_eq!(hit("busqué"), Some(LexemeId(4))); // -car spelling change
        assert_eq!(hit("gatos"), Some(LexemeId(5)));
        assert_eq!(hit("veces"), Some(LexemeId(6))); // -z → -ces plural
    }

    #[test]
    fn german_present_and_forms() {
        // Regular: machen → mache/machst/macht/machen/machen, + past participle.
        let machen = lex(1, "de", "machen", PartOfSpeech::Verb);
        let forms: Vec<_> = present_tense(&machen).into_iter().map(|c| c.form).collect();
        assert_eq!(forms, ["mache", "machst", "macht", "machen", "machen"]);
        assert_eq!(present_tense(&machen)[0].pronoun, "ich");
        let idx = build_form_index(&[machen.clone()]);
        assert_eq!(idx.get("macht"), Some(&LexemeId(1)));
        assert_eq!(idx.get("gemacht"), Some(&LexemeId(1)), "past participle resolves");

        // -t stem takes an epenthetic -e: arbeiten → du arbeitest, er arbeitet.
        let arbeiten = lex(2, "de", "arbeiten", PartOfSpeech::Verb);
        let af: Vec<_> = present_tense(&arbeiten).into_iter().map(|c| c.form).collect();
        assert_eq!(af, ["arbeite", "arbeitest", "arbeitet", "arbeiten", "arbeiten"]);

        // Irregular sein resolves and conjugates.
        let sein = lex(3, "de", "sein", PartOfSpeech::Verb);
        assert_eq!(present_tense(&sein)[2].form, "ist");
        let sidx = build_form_index(&[sein]);
        assert_eq!(sidx.get("bin"), Some(&LexemeId(3)));
        assert_eq!(sidx.get("ist"), Some(&LexemeId(3)));

        // Irregular cells are flagged; regular ones aren't.
        let essen = lex(4, "de", "essen", PartOfSpeech::Verb);
        let conj = present_tense(&essen);
        assert_eq!(conj[0].form, "esse");
        assert!(!conj[0].irregular, "ich esse follows the rule");
        assert_eq!(conj[2].form, "isst");
        assert!(conj[2].irregular, "er isst deviates from the rule (esst)");
        assert!(present_tense(&machen).iter().all(|c| !c.irregular), "machen is regular");
    }

    #[test]
    fn present_tense_tables() {
        // Irregular Spanish: ser → soy/eres/es/somos/son, labeled by pronoun.
        let ser = lex(2, "es", "ser", PartOfSpeech::Verb);
        let conj = present_tense(&ser);
        assert_eq!(conj.len(), 5);
        assert_eq!(conj[0].pronoun, "yo");
        assert_eq!(conj[0].gloss, "I");
        assert_eq!(conj[0].form, "soy");
        assert_eq!(conj[2].form, "es");

        // Regular -ar generates correctly: hablar → hablo … hablan.
        let hablar = lex(3, "es", "hablar", PartOfSpeech::Verb);
        let forms: Vec<_> = present_tense(&hablar).into_iter().map(|c| c.form).collect();
        assert_eq!(forms, ["hablo", "hablas", "habla", "hablamos", "hablan"]);

        // French -ger keeps the soft g in nous: manger → mangeons.
        let manger = lex(4, "fr", "manger", PartOfSpeech::Verb);
        assert_eq!(present_tense(&manger)[3].form, "mangeons");
        assert_eq!(present_tense(&manger)[0].pronoun, "je");

        // Non-verbs have no table.
        let gato = lex(5, "es", "gato", PartOfSpeech::Noun);
        assert!(present_tense(&gato).is_empty());
    }

    #[test]
    fn french_tenses_resolve() {
        let lexemes = vec![
            lex(1, "fr", "manger", PartOfSpeech::Verb),
            lex(2, "fr", "être", PartOfSpeech::Verb),
            lex(3, "fr", "aller", PartOfSpeech::Verb),
            lex(4, "fr", "livre", PartOfSpeech::Noun),
        ];
        let idx = build_form_index(&lexemes);
        let hit = |w: &str| idx.get(w).copied();
        for w in ["mange", "manges", "mangeait", "mangera", "mangé"] {
            assert_eq!(hit(w), Some(LexemeId(1)), "{w}");
        }
        for w in ["suis", "est", "était", "été"] {
            assert_eq!(hit(w), Some(LexemeId(2)), "{w}");
        }
        for w in ["vais", "va", "irai", "allé"] {
            assert_eq!(hit(w), Some(LexemeId(3)), "{w}");
        }
        assert_eq!(hit("livres"), Some(LexemeId(4)));
    }
}
