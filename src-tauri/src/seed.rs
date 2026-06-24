//! First-run seeding of the reference inventory.
//!
//! V1 ships a small, hand-curated Spanish frequency list (`seed/es_frequency.json`)
//! ordered most-frequent-first. Swap it for a larger, licensing-clean list
//! (spec §10, §11.2) by replacing that file — the id/rank are assigned by order.

use serde::Deserialize;

use glossa_core::{GrammarPattern, LanguageCode, Lexeme, LexemeId, PartOfSpeech, PatternId};
use glossa_storage::{StorageError, Store};

#[derive(Deserialize)]
struct SeedWord {
    lemma: String,
    pos: PartOfSpeech,
    gloss: String,
}

/// The bundled frequency list, embedded at compile time.
const ES_FREQUENCY: &str = include_str!("../seed/es_frequency.json");

/// Sync the reference inventory from the bundled seed on every launch.
///
/// Idempotent: lexeme ids are assigned by list order, so re-running just
/// updates lemma/pos/**gloss** for existing ids and adds any new words —
/// learner state keys on `lexeme_id` and is untouched. This means editing
/// `es_frequency.json` (e.g. adding meanings) reaches existing installs on the
/// next launch, instead of only seeding once on a fresh database.
pub async fn sync_inventory(store: &dyn Store, language: &LanguageCode) -> Result<(), StorageError> {
    let words: Vec<SeedWord> =
        serde_json::from_str(ES_FREQUENCY).expect("bundled es_frequency.json must be valid JSON");

    let lexemes: Vec<Lexeme> = words
        .into_iter()
        .enumerate()
        .map(|(i, w)| Lexeme {
            id: LexemeId(i as i64 + 1),
            language: language.clone(),
            lemma: w.lemma,
            pos: w.pos,
            frequency_rank: i as u32 + 1,
            gloss: Some(w.gloss),
        })
        .collect();

    store.upsert_lexemes(&lexemes).await?;
    store
        .upsert_grammar_patterns(&spanish_grammar(language))
        .await?;
    Ok(())
}

/// A small, ordered set of grammar patterns to target implicitly (spec §2.2).
fn spanish_grammar(language: &LanguageCode) -> Vec<GrammarPattern> {
    let p = |id: i64, label: &str, example: &str| GrammarPattern {
        id: PatternId(id),
        language: language.clone(),
        label: label.to_string(),
        example_template: example.to_string(),
    };
    vec![
        p(1, "gender-articles", "el libro, la casa, los niños, las mesas"),
        p(2, "present-regular-ar", "yo hablo, tú hablas, ella habla"),
        p(3, "present-regular-er-ir", "yo como, tú comes, ella vive"),
        p(4, "ser-vs-estar", "Soy estudiante. Estoy en casa."),
        p(5, "plural-nouns", "un gato, dos gatos; una flor, tres flores"),
        p(6, "preterite-regular-ar", "Ayer hablé y compré pan."),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_frequency_list_is_valid() {
        // Catches a bad POS value or malformed JSON in the seed file at test time
        // rather than at app startup.
        let words: Vec<SeedWord> =
            serde_json::from_str(ES_FREQUENCY).expect("es_frequency.json must deserialize");
        assert!(words.len() > 50, "seed list looks too small: {}", words.len());
        assert_eq!(words[0].lemma, "de", "list should be most-frequent-first");
    }
}
