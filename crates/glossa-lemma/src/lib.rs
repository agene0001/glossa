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

/// All surface forms we recognize for one lexeme (always includes the lemma).
pub fn surface_forms(lex: &Lexeme) -> Vec<String> {
    let lemma = lex.lemma.to_lowercase();
    let mut forms = vec![lemma.clone()];
    match lex.language.as_str() {
        "es" => spanish_forms(&lemma, lex.pos, &mut forms),
        "fr" => french_forms(&lemma, lex.pos, &mut forms),
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
