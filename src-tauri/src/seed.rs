//! First-run seeding of the reference inventory.
//!
//! V1 ships a small, hand-curated Spanish frequency list (`seed/es_frequency.json`)
//! ordered most-frequent-first. Swap it for a larger, licensing-clean list
//! (spec §10, §11.2) by replacing that file — the id/rank are assigned by order.

use std::collections::HashMap;

use serde::Deserialize;

use glossa_core::{
    ExampleSentence, GrammarPattern, LanguageCode, Lexeme, LexemeId, PartOfSpeech, PatternId, Unit,
    UnitId,
};
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

    let lemma_to_id: HashMap<String, LexemeId> =
        lexemes.iter().map(|l| (l.lemma.clone(), l.id)).collect();
    store
        .upsert_units(&spanish_units(language, &lemma_to_id))
        .await?;
    Ok(())
}

/// The starter roadmap: ordered, themed units with authored example sentences.
/// Words are referenced by lemma and resolved to ids from the seeded inventory.
fn spanish_units(language: &LanguageCode, ids: &HashMap<String, LexemeId>) -> Vec<Unit> {
    let resolve = |lemmas: &[&str]| -> Vec<LexemeId> {
        lemmas.iter().filter_map(|w| ids.get(*w).copied()).collect()
    };
    let ex = |text: &str, tr: &str| ExampleSentence {
        text: text.into(),
        translation: tr.into(),
    };

    vec![
        Unit {
            id: UnitId(1),
            language: language.clone(),
            title: "First Words & Greetings".into(),
            description: "Say hello, talk about yourself, and meet your first verb — ser (to be).".into(),
            target_lexemes: resolve(&["hola", "gracias", "yo", "tú", "ser", "no", "y", "amigo", "un"]),
            target_pattern: Some(PatternId(4)),
            examples: vec![
                ex("Hola. Yo soy un amigo.", "Hello. I am a friend."),
                ex("Gracias, amigo.", "Thank you, friend."),
                ex("Tú y yo.", "You and I."),
                ex("No, gracias.", "No, thank you."),
            ],
        },
        Unit {
            id: UnitId(2),
            language: language.clone(),
            title: "People & Family".into(),
            description: "People around you, and how Spanish marks gender with el / la.".into(),
            target_lexemes: resolve(&[
                "hombre", "mujer", "niño", "niña", "amiga", "familia", "padre", "madre", "el", "la",
            ]),
            target_pattern: Some(PatternId(1)),
            examples: vec![
                ex("El hombre y la mujer.", "The man and the woman."),
                ex(
                    "La familia: el padre, la madre, el niño y la niña.",
                    "The family: the father, the mother, the boy and the girl.",
                ),
                ex("El padre es un hombre.", "The father is a man."),
                ex("La madre es una mujer.", "The mother is a woman."),
            ],
        },
        Unit {
            id: UnitId(3),
            language: language.clone(),
            title: "Home & Things".into(),
            description: "Objects around the house — and making nouns plural.".into(),
            target_lexemes: resolve(&[
                "casa", "mesa", "puerta", "libro", "agua", "cosa", "gato", "perro",
            ]),
            target_pattern: Some(PatternId(5)),
            examples: vec![
                ex("La casa tiene una puerta.", "The house has a door."),
                ex("El libro está en la mesa.", "The book is on the table."),
                ex("Un gato y un perro.", "A cat and a dog."),
                ex("El agua está en la casa.", "The water is in the house."),
            ],
        },
        Unit {
            id: UnitId(4),
            language: language.clone(),
            title: "Eating & Drinking".into(),
            description: "Food and drink, with regular -er / -ir verbs like comer and beber.".into(),
            target_lexemes: resolve(&[
                "comer", "beber", "comida", "pan", "leche", "café", "manzana",
            ]),
            target_pattern: Some(PatternId(3)),
            examples: vec![
                ex("Yo como pan.", "I eat bread."),
                ex("Tú bebes leche.", "You drink milk."),
                ex("Ella come una manzana.", "She eats an apple."),
                ex("Yo bebo café y agua.", "I drink coffee and water."),
            ],
        },
        Unit {
            id: UnitId(5),
            language: language.clone(),
            title: "Everyday Actions".into(),
            description: "Common things you do, with regular -ar verbs like hablar and trabajar.".into(),
            target_lexemes: resolve(&[
                "hablar", "trabajar", "comprar", "hacer", "ir", "querer",
            ]),
            target_pattern: Some(PatternId(2)),
            examples: vec![
                ex("Yo hablo con un amigo.", "I speak with a friend."),
                ex("Ella trabaja en la ciudad.", "She works in the city."),
                ex("Nosotros compramos pan.", "We buy bread."),
                ex("Yo quiero comer.", "I want to eat."),
            ],
        },
        Unit {
            id: UnitId(6),
            language: language.clone(),
            title: "Time & Place".into(),
            description: "Talk about days and places — and your first taste of the past tense.".into(),
            target_lexemes: resolve(&[
                "día", "año", "noche", "mañana", "ciudad", "calle", "mundo", "tiempo",
            ]),
            target_pattern: Some(PatternId(6)),
            examples: vec![
                ex("El día es bueno.", "The day is good."),
                ex("La ciudad tiene calles.", "The city has streets."),
                ex("La noche y la mañana.", "The night and the morning."),
                ex("Ayer trabajé en la ciudad.", "Yesterday I worked in the city."),
            ],
        },
    ]
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
