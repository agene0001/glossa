//! `glossa-lemma` — resolve inflected surface forms back to their lexeme.
//!
//! V1 vocabulary is stored flat (one entry per lemma), but real text contains
//! conjugated verbs and plurals — `como`/`comió` for `comer`, `mange` for
//! `manger`, `gatos` for `gato`. Without this, those show as "unknown" even
//! when the learner knows the base word, which corrupts both highlighting and
//! the mastery graph.
//!
//! Approach (deliberately lightweight, no NLP dependency): generate the common
//! surface forms for each seeded lexeme — regular Spanish/French conjugation and
//! plural rules, plus a curated table for the high-frequency irregular verbs —
//! and build a `surface form → LexemeId` index. Missing a form just falls back
//! to "unknown" (same as today), so this is strictly an improvement.

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
    // Real lemmas win over any generated collision.
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
        if let Some(stem) = lemma.strip_suffix("ar") {
            add(forms, stem, &[
                "o", "as", "a", "amos", "áis", "an", // present
                "é", "aste", "ó", "amos", "asteis", "aron", // preterite
                "aba", "abas", "ábamos", "aban", // imperfect
                "ando", "ado", "ada", "ados", "adas", // gerund / participle
            ]);
        } else if let Some(stem) = lemma.strip_suffix("er").or_else(|| lemma.strip_suffix("ir")) {
            add(forms, stem, &[
                "o", "es", "e", "emos", "en", "imos", // present (-er/-ir)
                "í", "iste", "ió", "ieron", "isteis", // preterite
                "ía", "ías", "íamos", "ían", // imperfect
                "iendo", "ido", "ida", "idos", "idas", // gerund / participle
            ]);
        }
    } else if matches!(pos, PartOfSpeech::Noun | PartOfSpeech::Adjective) {
        if ends_with_vowel(lemma) {
            forms.push(format!("{lemma}s"));
        } else {
            forms.push(format!("{lemma}es"));
        }
        if pos == PartOfSpeech::Adjective {
            if let Some(base) = lemma.strip_suffix('o') {
                add(forms, base, &["a", "os", "as"]); // gender/number agreement
            }
        }
    }
}

/// Curated present + common past forms for high-frequency irregular Spanish verbs.
fn spanish_irregular(lemma: &str) -> Option<&'static [&'static str]> {
    let forms: &[&str] = match lemma {
        "ser" => &["soy", "eres", "es", "somos", "son", "fui", "fuiste", "fue", "fueron", "era", "eran"],
        "estar" => &["estoy", "estás", "está", "estamos", "están", "estaba", "estuvo"],
        "ir" => &["voy", "vas", "va", "vamos", "van", "fui", "fue", "fueron", "iba", "iban"],
        "haber" => &["he", "has", "ha", "hemos", "han", "hay", "había"],
        "tener" => &["tengo", "tienes", "tiene", "tenemos", "tienen", "tuve", "tuvo"],
        "hacer" => &["hago", "haces", "hace", "hacemos", "hacen", "hice", "hizo"],
        "poder" => &["puedo", "puedes", "puede", "podemos", "pueden", "pude", "pudo"],
        "querer" => &["quiero", "quieres", "quiere", "queremos", "quieren", "quise", "quiso"],
        "decir" => &["digo", "dices", "dice", "decimos", "dicen", "dije", "dijo"],
        "ver" => &["veo", "ves", "ve", "vemos", "ven", "vi", "vio"],
        "dar" => &["doy", "das", "da", "damos", "dan", "di", "dio"],
        "saber" => &["sé", "sabes", "sabe", "sabemos", "saben", "supe", "supo"],
        "venir" => &["vengo", "vienes", "viene", "venimos", "vienen", "vine", "vino"],
        "poner" => &["pongo", "pones", "pone", "ponemos", "ponen", "puse", "puso"],
        "salir" => &["salgo", "sales", "sale", "salimos", "salen", "salí"],
        "pensar" => &["pienso", "piensas", "piensa", "pensamos", "piensan", "pensé"],
        "volver" => &["vuelvo", "vuelves", "vuelve", "volvemos", "vuelven", "volví"],
        "encontrar" => &["encuentro", "encuentras", "encuentra", "encontramos", "encuentran"],
        "seguir" => &["sigo", "sigues", "sigue", "seguimos", "siguen"],
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
                "e", "es", "e", "ons", "ez", "ent", // present
                "é", "ée", "és", "ées", // past participle
                "ais", "ait", "aient", // imperfect
            ]);
        }
    } else if matches!(pos, PartOfSpeech::Noun | PartOfSpeech::Adjective) {
        // Plural: add -s unless it already ends in s/x/z.
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
        "être" => &["suis", "es", "est", "sommes", "êtes", "sont", "été", "était"],
        "avoir" => &["ai", "as", "a", "avons", "avez", "ont", "eu", "avait"],
        "aller" => &["vais", "vas", "va", "allons", "allez", "vont", "allé", "allée"],
        "faire" => &["fais", "fait", "faisons", "faites", "font", "faisait"],
        "vouloir" => &["veux", "veut", "voulons", "voulez", "veulent", "voulu"],
        "pouvoir" => &["peux", "peut", "pouvons", "pouvez", "peuvent", "pu"],
        "boire" => &["bois", "boit", "buvons", "buvez", "boivent", "bu"],
        "voir" => &["vois", "voit", "voyons", "voyez", "voient", "vu"],
        "venir" => &["viens", "vient", "venons", "venez", "viennent", "venu"],
        "dire" => &["dis", "dit", "disons", "dites", "disent"],
        "lire" => &["lis", "lit", "lisons", "lisez", "lisent", "lu"],
        "savoir" => &["sais", "sait", "savons", "savez", "savent", "su"],
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
    fn spanish_conjugations_and_plurals_resolve() {
        let lexemes = vec![
            lex(1, "es", "comer", PartOfSpeech::Verb),
            lex(2, "es", "ser", PartOfSpeech::Verb),
            lex(3, "es", "hablar", PartOfSpeech::Verb),
            lex(4, "es", "gato", PartOfSpeech::Noun),
        ];
        let idx = build_form_index(&lexemes);
        assert_eq!(idx.get("como"), Some(&LexemeId(1))); // comer
        assert_eq!(idx.get("comió"), Some(&LexemeId(1)));
        assert_eq!(idx.get("soy"), Some(&LexemeId(2))); // ser (irregular)
        assert_eq!(idx.get("es"), Some(&LexemeId(2)));
        assert_eq!(idx.get("hablo"), Some(&LexemeId(3))); // hablar
        assert_eq!(idx.get("hablé"), Some(&LexemeId(3)));
        assert_eq!(idx.get("gatos"), Some(&LexemeId(4))); // plural
        assert_eq!(idx.get("gato"), Some(&LexemeId(4))); // lemma itself
    }

    #[test]
    fn french_conjugations_resolve() {
        let lexemes = vec![
            lex(1, "fr", "manger", PartOfSpeech::Verb),
            lex(2, "fr", "être", PartOfSpeech::Verb),
            lex(3, "fr", "livre", PartOfSpeech::Noun),
        ];
        let idx = build_form_index(&lexemes);
        assert_eq!(idx.get("mange"), Some(&LexemeId(1)));
        assert_eq!(idx.get("manges"), Some(&LexemeId(1)));
        assert_eq!(idx.get("suis"), Some(&LexemeId(2))); // être
        assert_eq!(idx.get("est"), Some(&LexemeId(2)));
        assert_eq!(idx.get("livres"), Some(&LexemeId(3)));
    }
}
