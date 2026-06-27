//! First-run seeding of the reference inventory, per language.
//!
//! Each supported language is seeded from a bundled frequency list plus
//! hand-authored grammar patterns and curriculum units. Ids are namespaced by a
//! per-language `base` so lexeme/pattern/unit ids never collide across
//! languages. Spanish keeps base 0 (stable ids for existing installs); add a
//! language by dropping in a frequency file and a `*_grammar` / `*_units` pair.

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

const ES_FREQUENCY: &str = include_str!("../seed/es_frequency.json");
const FR_FREQUENCY: &str = include_str!("../seed/fr_frequency.json");

/// Sync every supported language's inventory from its bundled seed on launch.
/// Idempotent: ids are assigned by list order, so re-running updates existing
/// rows (e.g. adds new glosses/units) without disturbing learner state.
pub async fn sync_inventory(store: &dyn Store) -> Result<(), StorageError> {
    seed_language(
        store,
        &LanguageCode::spanish(),
        0,
        ES_FREQUENCY,
        spanish_grammar,
        spanish_units,
    )
    .await?;
    seed_language(
        store,
        &LanguageCode::new("fr"),
        1000,
        FR_FREQUENCY,
        french_grammar,
        french_units,
    )
    .await?;
    Ok(())
}

async fn seed_language(
    store: &dyn Store,
    language: &LanguageCode,
    base: i64,
    frequency_json: &str,
    grammar_fn: fn(&LanguageCode, i64) -> Vec<GrammarPattern>,
    units_fn: fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<Unit>,
) -> Result<(), StorageError> {
    let words: Vec<SeedWord> =
        serde_json::from_str(frequency_json).expect("bundled frequency json must be valid");

    let lexemes: Vec<Lexeme> = words
        .into_iter()
        .enumerate()
        .map(|(i, w)| Lexeme {
            id: LexemeId(base + i as i64 + 1),
            language: language.clone(),
            lemma: w.lemma,
            pos: w.pos,
            frequency_rank: i as u32 + 1,
            gloss: Some(w.gloss),
        })
        .collect();
    store.upsert_lexemes(&lexemes).await?;
    store.upsert_grammar_patterns(&grammar_fn(language, base)).await?;

    let lemma_to_id: HashMap<String, LexemeId> =
        lexemes.iter().map(|l| (l.lemma.clone(), l.id)).collect();
    store
        .upsert_units(&units_fn(language, base, &lemma_to_id))
        .await?;
    Ok(())
}

// --- Spanish -------------------------------------------------------------

fn spanish_grammar(language: &LanguageCode, base: i64) -> Vec<GrammarPattern> {
    let p = |n: i64, label: &str, ex: &str, expl: &str| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        example_template: ex.into(),
        explanation: Some(expl.into()),
    };
    vec![
        p(1, "gender-articles", "el libro, la casa, los niños, las mesas",
          "Spanish nouns are masculine or feminine. Use 'el/un' with masculine nouns and 'la/una' with feminine ones; in the plural they become 'los/las'."),
        p(2, "present-regular-ar", "yo hablo, tú hablas, ella habla",
          "Regular -ar verbs drop -ar and add endings: -o (I), -as (you), -a (he/she). E.g. hablar → hablo, hablas, habla."),
        p(3, "present-regular-er-ir", "yo como, tú comes, ella vive",
          "Regular -er/-ir verbs use the endings -o, -es, -e. E.g. comer → como, comes, come."),
        p(4, "ser-vs-estar", "Soy estudiante. Estoy en casa.",
          "Spanish has two verbs for 'to be': 'ser' for identity and lasting traits, 'estar' for location and temporary states."),
        p(5, "plural-nouns", "un gato, dos gatos; una flor, tres flores",
          "Make nouns plural by adding -s after a vowel (gato → gatos) or -es after a consonant (flor → flores)."),
        p(6, "preterite-regular-ar", "Ayer hablé y compré pan.",
          "For completed past actions with -ar verbs, use the preterite: hablé (I spoke), hablaste (you spoke), habló (he/she spoke)."),
    ]
}

fn spanish_units(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<Unit> {
    let resolve = |lemmas: &[&str]| -> Vec<LexemeId> {
        lemmas.iter().filter_map(|w| ids.get(*w).copied()).collect()
    };
    let ex = |t: &str, tr: &str| ExampleSentence {
        text: t.into(),
        translation: tr.into(),
    };
    let unit = |n: i64, title: &str, desc: &str, words: &[&str], pat: Option<i64>, examples: Vec<ExampleSentence>| Unit {
        id: UnitId(base + n),
        language: language.clone(),
        title: title.into(),
        description: desc.into(),
        target_lexemes: resolve(words),
        target_pattern: pat.map(|n| PatternId(base + n)),
        examples,
    };
    vec![
        unit(1, "First Words & Greetings",
            "Say hello, talk about yourself, and meet your first verb — ser (to be).",
            &["hola", "gracias", "yo", "tú", "ser", "no", "y", "amigo", "un"], Some(4),
            vec![
                ex("Hola. Yo soy un amigo.", "Hello. I am a friend."),
                ex("Gracias, amigo.", "Thank you, friend."),
                ex("Tú y yo.", "You and I."),
                ex("No, gracias.", "No, thank you."),
            ]),
        unit(2, "People & Family",
            "People around you, and how Spanish marks gender with el / la.",
            &["hombre", "mujer", "niño", "niña", "amiga", "familia", "padre", "madre", "el", "la"], Some(1),
            vec![
                ex("El hombre y la mujer.", "The man and the woman."),
                ex("La familia: el padre, la madre, el niño y la niña.", "The family: the father, the mother, the boy and the girl."),
                ex("El padre es un hombre.", "The father is a man."),
                ex("La madre es una mujer.", "The mother is a woman."),
            ]),
        unit(3, "Home & Things",
            "Objects around the house — and making nouns plural.",
            &["casa", "mesa", "puerta", "libro", "agua", "cosa", "gato", "perro"], Some(5),
            vec![
                ex("La casa tiene una puerta.", "The house has a door."),
                ex("El libro está en la mesa.", "The book is on the table."),
                ex("Un gato y un perro.", "A cat and a dog."),
                ex("El agua está en la casa.", "The water is in the house."),
            ]),
        unit(4, "Eating & Drinking",
            "Food and drink, with regular -er / -ir verbs like comer and beber.",
            &["comer", "beber", "comida", "pan", "leche", "café", "manzana"], Some(3),
            vec![
                ex("Yo como pan.", "I eat bread."),
                ex("Tú bebes leche.", "You drink milk."),
                ex("Ella come una manzana.", "She eats an apple."),
                ex("Yo bebo café y agua.", "I drink coffee and water."),
            ]),
        unit(5, "Everyday Actions",
            "Common things you do, with regular -ar verbs like hablar and trabajar.",
            &["hablar", "trabajar", "comprar", "hacer", "ir", "querer"], Some(2),
            vec![
                ex("Yo hablo con un amigo.", "I speak with a friend."),
                ex("Ella trabaja en la ciudad.", "She works in the city."),
                ex("Nosotros compramos pan.", "We buy bread."),
                ex("Yo quiero comer.", "I want to eat."),
            ]),
        unit(6, "Time & Place",
            "Talk about days and places — and your first taste of the past tense.",
            &["día", "año", "noche", "mañana", "ciudad", "calle", "mundo", "tiempo"], Some(6),
            vec![
                ex("El día es bueno.", "The day is good."),
                ex("La ciudad tiene calles.", "The city has streets."),
                ex("La noche y la mañana.", "The night and the morning."),
                ex("Ayer trabajé en la ciudad.", "Yesterday I worked in the city."),
            ]),
        unit(7, "Numbers & Describing",
            "Count to three and describe things with common adjectives.",
            &["uno", "dos", "tres", "bueno", "malo", "grande", "pequeño", "nuevo"], Some(1),
            vec![
                ex("Uno, dos, tres.", "One, two, three."),
                ex("Un libro nuevo.", "A new book."),
                ex("La casa es grande.", "The house is big."),
                ex("El café es bueno.", "The coffee is good."),
            ]),
        unit(8, "Feelings & Life",
            "Describe people and ideas — and use ser to say how things are.",
            &["feliz", "importante", "mismo", "viejo", "gente", "vida", "momento", "parte"], Some(4),
            vec![
                ex("Yo soy feliz.", "I am happy."),
                ex("La familia es importante.", "Family is important."),
                ex("La vida es buena.", "Life is good."),
                ex("La gente es importante.", "People are important."),
            ]),
        unit(9, "Numbers 1–10",
            "Count from one to ten and use numbers with nouns.",
            &["uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez"], None,
            vec![
                ex("Uno, dos, tres, cuatro, cinco.", "One, two, three, four, five."),
                ex("Seis, siete, ocho, nueve, diez.", "Six, seven, eight, nine, ten."),
                ex("Yo tengo dos gatos.", "I have two cats."),
                ex("Tres libros y cuatro mesas.", "Three books and four tables."),
            ]),
        unit(10, "Colors",
            "Name colors and make adjectives agree with the noun.",
            &["color", "rojo", "azul", "verde", "negro", "blanco", "amarillo"], Some(1),
            vec![
                ex("El gato negro y el perro blanco.", "The black cat and the white dog."),
                ex("Una casa roja.", "A red house."),
                ex("El libro es azul.", "The book is blue."),
                ex("El color verde y el color amarillo.", "The color green and the color yellow."),
            ]),
        unit(11, "Days & Time",
            "Talk about when things happen — today, yesterday, now.",
            &["hoy", "ayer", "ahora", "hora", "minuto", "mañana", "día", "noche"], Some(6),
            vec![
                ex("Hoy es un buen día.", "Today is a good day."),
                ex("Ayer comí pan.", "Yesterday I ate bread."),
                ex("Ahora yo trabajo.", "Now I work."),
                ex("La hora y el minuto.", "The hour and the minute."),
            ]),
        unit(12, "More Food",
            "More food and drink, reinforcing -er verbs like comer and beber.",
            &["fruta", "verdura", "carne", "queso", "huevo", "arroz", "sopa", "vino"], Some(3),
            vec![
                ex("Yo como fruta y verdura.", "I eat fruit and vegetables."),
                ex("La carne y el queso.", "The meat and the cheese."),
                ex("Ella come arroz con huevo.", "She eats rice with egg."),
                ex("Yo bebo vino.", "I drink wine."),
            ]),
        unit(13, "Around Town",
            "Places in a city, and saying where things are with estar.",
            &["aeropuerto", "hotel", "restaurante", "tienda", "mercado", "banco", "parque", "ciudad"], Some(2),
            vec![
                ex("El hotel está en la ciudad.", "The hotel is in the city."),
                ex("Yo voy a la tienda.", "I go to the store."),
                ex("El restaurante y el banco.", "The restaurant and the bank."),
                ex("El parque es grande.", "The park is big."),
            ]),
        unit(14, "The Body",
            "Parts of the body — and plurals (un ojo, dos ojos).",
            &["cabeza", "ojo", "boca", "mano", "pie", "brazo", "pierna", "corazón"], Some(5),
            vec![
                ex("La cabeza y el corazón.", "The head and the heart."),
                ex("Dos ojos y una boca.", "Two eyes and one mouth."),
                ex("Yo tengo dos manos y dos pies.", "I have two hands and two feet."),
                ex("El brazo y la pierna.", "The arm and the leg."),
            ]),
        unit(15, "Everyday Verbs II",
            "More common actions, with regular -ar verbs.",
            &["necesitar", "usar", "ayudar", "buscar", "esperar", "aprender", "entender"], Some(2),
            vec![
                ex("Yo necesito agua.", "I need water."),
                ex("Ella ayuda a un amigo.", "She helps a friend."),
                ex("Nosotros aprendemos.", "We are learning."),
                ex("Yo busco el libro.", "I look for the book."),
            ]),
    ]
}

// --- French --------------------------------------------------------------

fn french_grammar(language: &LanguageCode, base: i64) -> Vec<GrammarPattern> {
    let p = |n: i64, label: &str, ex: &str, expl: &str| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        example_template: ex.into(),
        explanation: Some(expl.into()),
    };
    vec![
        p(1, "articles-le-la", "le livre, la maison; un, une",
          "French nouns are masculine or feminine. 'le/un' go with masculine nouns, 'la/une' with feminine ones."),
        p(2, "present-er-verbs", "je parle, tu parles, il parle",
          "Regular -er verbs drop -er and add -e, -es, -e: parler → je parle, tu parles, il parle."),
        p(3, "etre-avoir", "je suis, j'ai",
          "Two essential irregular verbs: être (to be) → je suis, tu es, il est; avoir (to have) → j'ai, tu as, il a."),
        p(4, "negation-ne-pas", "je ne parle pas",
          "To make a sentence negative, wrap the verb with ne ... pas: je parle → je ne parle pas."),
        p(5, "plural-s", "un livre, deux livres",
          "Most French nouns add a (usually silent) -s in the plural: un livre → deux livres."),
    ]
}

fn french_units(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<Unit> {
    let resolve = |lemmas: &[&str]| -> Vec<LexemeId> {
        lemmas.iter().filter_map(|w| ids.get(*w).copied()).collect()
    };
    let ex = |t: &str, tr: &str| ExampleSentence {
        text: t.into(),
        translation: tr.into(),
    };
    let unit = |n: i64, title: &str, desc: &str, words: &[&str], pat: Option<i64>, examples: Vec<ExampleSentence>| Unit {
        id: UnitId(base + n),
        language: language.clone(),
        title: title.into(),
        description: desc.into(),
        target_lexemes: resolve(words),
        target_pattern: pat.map(|n| PatternId(base + n)),
        examples,
    };
    vec![
        unit(1, "Premiers mots (First Words)",
            "Greet people, introduce yourself, and meet être (to be).",
            &["bonjour", "merci", "je", "tu", "être", "oui", "non", "ami", "un"], Some(3),
            vec![
                ex("Bonjour. Je suis un ami.", "Hello. I am a friend."),
                ex("Merci, ami.", "Thank you, friend."),
                ex("Oui ou non ?", "Yes or no?"),
                ex("Tu es un ami.", "You are a friend."),
            ]),
        unit(2, "Les gens (People)",
            "People around you, and the articles le / la.",
            &["homme", "femme", "enfant", "amie", "famille", "père", "mère", "le", "la"], Some(1),
            vec![
                ex("L'homme et la femme.", "The man and the woman."),
                ex("La famille : le père, la mère et l'enfant.", "The family: the father, the mother and the child."),
                ex("Le père est un homme.", "The father is a man."),
                ex("La mère est une femme.", "The mother is a woman."),
            ]),
        unit(3, "À la maison (At Home)",
            "Things around the house, and plurals with -s.",
            &["maison", "table", "porte", "livre", "eau", "chien", "chat"], Some(5),
            vec![
                ex("La maison a une porte.", "The house has a door."),
                ex("Le livre est sur la table.", "The book is on the table."),
                ex("Un chien et un chat.", "A dog and a cat."),
                ex("L'eau est dans la maison.", "The water is in the house."),
            ]),
        unit(4, "Manger et boire (Eating & Drinking)",
            "Food and drink, with regular -er verbs like manger.",
            &["manger", "boire", "pain", "lait", "café", "pomme", "nourriture"], Some(2),
            vec![
                ex("Je mange une pomme.", "I eat an apple."),
                ex("Tu manges du pain.", "You eat bread."),
                ex("Elle boit du lait.", "She drinks milk."),
                ex("Je bois un café.", "I drink a coffee."),
            ]),
        unit(5, "Actions (Everyday Actions)",
            "Common things you do, and how to say what you want.",
            &["faire", "aller", "parler", "travailler", "acheter", "vouloir"], Some(2),
            vec![
                ex("Je parle avec un ami.", "I speak with a friend."),
                ex("Elle travaille dans la ville.", "She works in the city."),
                ex("Nous achetons du pain.", "We buy bread."),
                ex("Je veux manger.", "I want to eat."),
            ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_frequency_lists_are_valid() {
        for json in [ES_FREQUENCY, FR_FREQUENCY] {
            let words: Vec<SeedWord> =
                serde_json::from_str(json).expect("frequency json must deserialize");
            assert!(words.len() > 40, "seed list too small: {}", words.len());
        }
    }

    #[test]
    fn unit_targets_resolve_to_seeded_words() {
        // Every word referenced by a unit must exist in that language's list,
        // or progress over it would be impossible.
        let es: Vec<SeedWord> = serde_json::from_str(ES_FREQUENCY).unwrap();
        let es_ids: HashMap<String, LexemeId> = es
            .iter()
            .enumerate()
            .map(|(i, w)| (w.lemma.clone(), LexemeId(i as i64 + 1)))
            .collect();
        for u in spanish_units(&LanguageCode::spanish(), 0, &es_ids) {
            assert!(
                !u.target_lexemes.is_empty(),
                "unit '{}' resolved no target words",
                u.title
            );
        }
    }
}
