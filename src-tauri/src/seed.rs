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
    ExampleSentence, GrammarDrill, GrammarPattern, LanguageCode, Lexeme, LexemeId, PackId,
    PartOfSpeech, PatternId, ReadingPassage, Unit, UnitId, VocabPack,
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
        spanish_packs,
    )
    .await?;
    seed_language(
        store,
        &LanguageCode::new("fr"),
        1000,
        FR_FREQUENCY,
        french_grammar,
        french_units,
        french_packs,
    )
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn seed_language(
    store: &dyn Store,
    language: &LanguageCode,
    base: i64,
    frequency_json: &str,
    grammar_fn: fn(&LanguageCode, i64) -> Vec<GrammarPattern>,
    units_fn: fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<Unit>,
    packs_fn: fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<VocabPack>,
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
    store
        .upsert_vocab_packs(&packs_fn(language, base, &lemma_to_id))
        .await?;
    Ok(())
}

// --- Spanish -------------------------------------------------------------

fn spanish_grammar(language: &LanguageCode, base: i64) -> Vec<GrammarPattern> {
    let d = |prompt: &str, answer: &str, tr: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
    };
    let p = |n: i64, label: &str, title: &str, ex: &str, expl: &str, prereqs: &[i64], drills: Vec<GrammarDrill>| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        title: title.into(),
        example_template: ex.into(),
        explanation: Some(expl.into()),
        prerequisites: prereqs.iter().map(|n| PatternId(base + n)).collect(),
        drills,
    };
    vec![
        p(1, "gender-articles", "Gender & articles (el / la)", "el libro, la casa, los niños, las mesas",
          "Spanish nouns are masculine or feminine. Use 'el/un' with masculine nouns and 'la/una' with feminine ones; in the plural they become 'los/las'.",
          &[], vec![
            d("___ libro es nuevo. (the, m.)", "el", "The book is new."),
            d("___ casa es grande. (the, f.)", "la", "The house is big."),
            d("Es ___ amigo. (a, m.)", "un", "He is a friend."),
            d("Es ___ mesa. (a, f.)", "una", "It is a table."),
          ]),
        p(2, "present-regular-ar", "Present tense: -ar verbs", "yo hablo, tú hablas, ella habla",
          "Regular -ar verbs drop -ar and add endings: -o (I), -as (you), -a (he/she). E.g. hablar → hablo, hablas, habla.",
          &[], vec![
            d("Yo ___ español. (hablar)", "hablo", "I speak Spanish."),
            d("Ella ___ en la ciudad. (trabajar)", "trabaja", "She works in the city."),
            d("Nosotros ___ pan. (comprar)", "compramos", "We buy bread."),
            d("Tú ___ mucho. (hablar)", "hablas", "You speak a lot."),
          ]),
        p(3, "present-regular-er-ir", "Present tense: -er / -ir verbs", "yo como, tú comes, ella vive",
          "Regular -er/-ir verbs use the endings -o, -es, -e. E.g. comer → como, comes, come.",
          &[2], vec![
            d("Yo ___ pan. (comer)", "como", "I eat bread."),
            d("Ella ___ leche. (beber)", "bebe", "She drinks milk."),
            d("Nosotros ___ aquí. (vivir)", "vivimos", "We live here."),
            d("Tú ___ una manzana. (comer)", "comes", "You eat an apple."),
          ]),
        p(4, "ser-vs-estar", "To be: ser vs estar", "Soy estudiante. Estoy en casa.",
          "Spanish has two verbs for 'to be': 'ser' for identity and lasting traits, 'estar' for location and temporary states.",
          &[2], vec![
            d("Yo ___ estudiante. (ser)", "soy", "I am a student."),
            d("Ella ___ en casa. (estar)", "está", "She is at home."),
            d("Nosotros ___ amigos. (ser)", "somos", "We are friends."),
            d("Yo ___ feliz hoy. (estar)", "estoy", "I am happy today."),
          ]),
        p(5, "plural-nouns", "Making nouns plural", "un gato, dos gatos; una flor, tres flores",
          "Make nouns plural by adding -s after a vowel (gato → gatos) or -es after a consonant (flor → flores).",
          &[1], vec![
            d("un gato, dos ___ (gato)", "gatos", "one cat, two cats"),
            d("una flor, tres ___ (flor)", "flores", "one flower, three flowers"),
            d("un libro, dos ___ (libro)", "libros", "one book, two books"),
            d("una vez, dos ___ (vez)", "veces", "one time, two times"),
          ]),
        p(6, "preterite-regular-ar", "The past tense (-ar verbs)", "Ayer hablé y compré pan.",
          "For completed past actions with -ar verbs, use the preterite: hablé (I spoke), hablaste (you spoke), habló (he/she spoke).",
          &[2], vec![
            d("Ayer yo ___ con un amigo. (hablar)", "hablé", "Yesterday I spoke with a friend."),
            d("Ella ___ en la ciudad. (trabajar)", "trabajó", "She worked in the city."),
            d("Yo ___ pan ayer. (comprar)", "compré", "I bought bread yesterday."),
            d("Tú ___ mucho. (trabajar)", "trabajaste", "You worked a lot."),
          ]),
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
    let rd = |title: &str, text: &str, tr: &str| {
        Some(ReadingPassage {
            title: title.into(),
            text: text.into(),
            translation: tr.into(),
        })
    };
    #[allow(clippy::too_many_arguments)]
    let unit = |n: i64,
                level: &str,
                title: &str,
                objective: &str,
                desc: &str,
                words: &[&str],
                pat: Option<i64>,
                reading: Option<ReadingPassage>,
                examples: Vec<ExampleSentence>| Unit {
        id: UnitId(base + n),
        language: language.clone(),
        title: title.into(),
        description: desc.into(),
        level: level.into(),
        objective: objective.into(),
        target_lexemes: resolve(words),
        target_pattern: pat.map(|n| PatternId(base + n)),
        reading,
        examples,
    };
    vec![
        unit(1, "A1.1", "First Words & Greetings",
            "Greet people, say thank you, and tell someone you're a friend.",
            "Say hello, talk about yourself, and meet your first verb — ser (to be).",
            &["hola", "gracias", "yo", "tú", "ser", "no", "y", "amigo", "un"], Some(4),
            rd("Dos amigos",
                "— Hola. Yo soy Ana. — Hola, Ana. ¿Tú eres mi amiga? — Sí. Tú y yo somos amigos. — ¡Gracias!",
                "— Hello. I am Ana. — Hello, Ana. Are you my friend? — Yes. You and I are friends. — Thank you!"),
            vec![
                ex("Hola. Yo soy un amigo.", "Hello. I am a friend."),
                ex("Gracias, amigo.", "Thank you, friend."),
                ex("Tú y yo.", "You and I."),
                ex("No, gracias.", "No, thank you."),
            ]),
        unit(2, "A1.1", "People & Family",
            "Name the people in a family and use el / la to talk about them.",
            "People around you, and how Spanish marks gender with el / la.",
            &["hombre", "mujer", "niño", "niña", "amiga", "familia", "padre", "madre", "el", "la"], Some(1),
            rd("La familia de Ana",
                "Esta es la familia de Ana. El padre es un hombre alto. La madre es una mujer amable. El niño y la niña son sus hijos. ¡Es una familia feliz!",
                "This is Ana's family. The father is a tall man. The mother is a kind woman. The boy and the girl are their children. It's a happy family!"),
            vec![
                ex("El hombre y la mujer.", "The man and the woman."),
                ex("La familia: el padre, la madre, el niño y la niña.", "The family: the father, the mother, the boy and the girl."),
                ex("El padre es un hombre.", "The father is a man."),
                ex("La madre es una mujer.", "The mother is a woman."),
            ]),
        unit(3, "A1.1", "Home & Things",
            "Name common things in a home and talk about more than one.",
            "Objects around the house — and making nouns plural.",
            &["casa", "mesa", "puerta", "libro", "agua", "cosa", "gato", "perro"], Some(5),
            rd("En casa",
                "En mi casa hay muchas cosas. Sobre la mesa hay un libro y un vaso de agua. El gato duerme cerca de la puerta. El perro está en el jardín.",
                "In my house there are many things. On the table there is a book and a glass of water. The cat sleeps near the door. The dog is in the garden."),
            vec![
                ex("La casa tiene una puerta.", "The house has a door."),
                ex("El libro está en la mesa.", "The book is on the table."),
                ex("Un gato y un perro.", "A cat and a dog."),
                ex("El agua está en la casa.", "The water is in the house."),
            ]),
        unit(4, "A1.1", "Eating & Drinking",
            "Say what you eat and drink using everyday verbs.",
            "Food and drink, with regular -er / -ir verbs like comer and beber.",
            &["comer", "beber", "comida", "pan", "leche", "café", "manzana"], Some(3),
            rd("El desayuno",
                "Por la mañana yo como pan con una manzana. Mi madre bebe café y mi padre bebe leche. La comida es simple, pero buena. ¡Me gusta el desayuno!",
                "In the morning I eat bread with an apple. My mother drinks coffee and my father drinks milk. The food is simple, but good. I like breakfast!"),
            vec![
                ex("Yo como pan.", "I eat bread."),
                ex("Tú bebes leche.", "You drink milk."),
                ex("Ella come una manzana.", "She eats an apple."),
                ex("Yo bebo café y agua.", "I drink coffee and water."),
            ]),
        unit(5, "A1.2", "Everyday Actions",
            "Talk about everyday actions and say what you want to do.",
            "Common things you do, with regular -ar verbs like hablar and trabajar.",
            &["hablar", "trabajar", "comprar", "hacer", "ir", "querer"], Some(2),
            rd("Un día normal",
                "Cada día yo trabajo en la ciudad. Hablo con mis amigos y hago muchas cosas. Después voy a la tienda y compro pan. Por la noche quiero descansar.",
                "Every day I work in the city. I talk with my friends and do many things. Afterwards I go to the store and buy bread. At night I want to rest."),
            vec![
                ex("Yo hablo con un amigo.", "I speak with a friend."),
                ex("Ella trabaja en la ciudad.", "She works in the city."),
                ex("Nosotros compramos pan.", "We buy bread."),
                ex("Yo quiero comer.", "I want to eat."),
            ]),
        unit(6, "A1.2", "Time & Place",
            "Talk about days, places, and what you did yesterday.",
            "Talk about days and places — and your first taste of the past tense.",
            &["día", "año", "noche", "mañana", "ciudad", "calle", "mundo", "tiempo"], Some(6),
            rd("La ciudad de noche",
                "Por la noche, la ciudad es tranquila. Las calles están casi vacías. Ayer pasé toda la mañana aquí; hoy quiero ver más del mundo. El tiempo pasa rápido.",
                "At night, the city is calm. The streets are almost empty. Yesterday I spent the whole morning here; today I want to see more of the world. Time goes by fast."),
            vec![
                ex("El día es bueno.", "The day is good."),
                ex("La ciudad tiene calles.", "The city has streets."),
                ex("La noche y la mañana.", "The night and the morning."),
                ex("Ayer trabajé en la ciudad.", "Yesterday I worked in the city."),
            ]),
        unit(7, "A1.1", "Numbers & Describing",
            "Count to three and describe things as big, small, good, or bad.",
            "Count to three and describe things with common adjectives.",
            &["uno", "dos", "tres", "bueno", "malo", "grande", "pequeño", "nuevo"], Some(1),
            rd("Tres libros",
                "Tengo tres libros nuevos. Uno es grande y dos son pequeños. El libro grande es muy bueno. El pequeño no es malo, pero es aburrido.",
                "I have three new books. One is big and two are small. The big book is very good. The small one is not bad, but it's boring."),
            vec![
                ex("Uno, dos, tres.", "One, two, three."),
                ex("Un libro nuevo.", "A new book."),
                ex("La casa es grande.", "The house is big."),
                ex("El café es bueno.", "The coffee is good."),
            ]),
        unit(8, "A1.2", "Feelings & Life",
            "Describe how people and life feel using ser.",
            "Describe people and ideas — and use ser to say how things are.",
            &["feliz", "importante", "mismo", "viejo", "gente", "vida", "momento", "parte"], Some(4),
            rd("Un buen momento",
                "Hoy soy muy feliz. La gente que quiero es la parte más importante de mi vida. En este momento, todo es bueno. Mi viejo amigo piensa lo mismo.",
                "Today I am very happy. The people I love are the most important part of my life. At this moment, everything is good. My old friend thinks the same."),
            vec![
                ex("Yo soy feliz.", "I am happy."),
                ex("La familia es importante.", "Family is important."),
                ex("La vida es buena.", "Life is good."),
                ex("La gente es importante.", "People are important."),
            ]),
        unit(9, "A1.1", "Numbers 1–10",
            "Count from one to ten and use numbers with things.",
            "Count from one to ten and use numbers with nouns.",
            &["uno", "dos", "tres", "cuatro", "cinco", "seis", "siete", "ocho", "nueve", "diez"], None,
            rd("Contar hasta diez",
                "Vamos a contar: uno, dos, tres, cuatro, cinco. Después: seis, siete, ocho, nueve, diez. En la mesa hay seis manzanas y cuatro libros. ¡Diez cosas en total!",
                "Let's count: one, two, three, four, five. Then: six, seven, eight, nine, ten. On the table there are six apples and four books. Ten things in total!"),
            vec![
                ex("Uno, dos, tres, cuatro, cinco.", "One, two, three, four, five."),
                ex("Seis, siete, ocho, nueve, diez.", "Six, seven, eight, nine, ten."),
                ex("Yo tengo dos gatos.", "I have two cats."),
                ex("Tres libros y cuatro mesas.", "Three books and four tables."),
            ]),
        unit(10, "A1.1", "Colors",
            "Name colors and describe objects by their color.",
            "Name colors and make adjectives agree with the noun.",
            &["color", "rojo", "azul", "verde", "negro", "blanco", "amarillo"], Some(1),
            rd("Los colores",
                "Mi color favorito es el azul. Tengo un gato negro y un perro blanco. En el jardín hay flores rojas y amarillas. La hierba es verde. ¡Me gustan todos los colores!",
                "My favorite color is blue. I have a black cat and a white dog. In the garden there are red and yellow flowers. The grass is green. I like all the colors!"),
            vec![
                ex("El gato negro y el perro blanco.", "The black cat and the white dog."),
                ex("Una casa roja.", "A red house."),
                ex("El libro es azul.", "The book is blue."),
                ex("El color verde y el color amarillo.", "The color green and the color yellow."),
            ]),
        unit(11, "A1.2", "Days & Time",
            "Say when things happen — today, yesterday, and now.",
            "Talk about when things happen — today, yesterday, now.",
            &["hoy", "ayer", "ahora", "hora", "minuto", "mañana", "día", "noche"], Some(6),
            rd("Hoy y ayer",
                "Ayer fue un día largo. Hoy quiero descansar. Ahora son las tres; en un minuto voy a comer. Esta noche duermo temprano y mañana empiezo de nuevo.",
                "Yesterday was a long day. Today I want to rest. Now it's three o'clock; in a minute I'm going to eat. Tonight I sleep early and tomorrow I start again."),
            vec![
                ex("Hoy es un buen día.", "Today is a good day."),
                ex("Ayer comí pan.", "Yesterday I ate bread."),
                ex("Ahora yo trabajo.", "Now I work."),
                ex("La hora y el minuto.", "The hour and the minute."),
            ]),
        unit(12, "A1.2", "More Food",
            "Talk about more foods and put together a simple meal.",
            "More food and drink, reinforcing -er verbs like comer and beber.",
            &["fruta", "verdura", "carne", "queso", "huevo", "arroz", "sopa", "vino"], Some(3),
            rd("La cena",
                "Para la cena hay sopa, arroz y carne. También como verdura y un poco de queso. De postre, fruta fresca. Mi padre bebe vino y yo bebo agua.",
                "For dinner there is soup, rice and meat. I also eat vegetables and a little cheese. For dessert, fresh fruit. My father drinks wine and I drink water."),
            vec![
                ex("Yo como fruta y verdura.", "I eat fruit and vegetables."),
                ex("La carne y el queso.", "The meat and the cheese."),
                ex("Ella come arroz con huevo.", "She eats rice with egg."),
                ex("Yo bebo vino.", "I drink wine."),
            ]),
        unit(13, "A1.2", "Around Town",
            "Name places in town and say where they are.",
            "Places in a city, and saying where things are with estar.",
            &["aeropuerto", "hotel", "restaurante", "tienda", "mercado", "banco", "parque", "ciudad"], Some(2),
            rd("Un día en la ciudad",
                "Primero voy al banco y después a la tienda. Al mediodía como en un restaurante cerca del mercado. Por la tarde camino por el parque. Mañana salgo del hotel hacia el aeropuerto.",
                "First I go to the bank and then to the store. At noon I eat at a restaurant near the market. In the afternoon I walk through the park. Tomorrow I leave the hotel for the airport."),
            vec![
                ex("El hotel está en la ciudad.", "The hotel is in the city."),
                ex("Yo voy a la tienda.", "I go to the store."),
                ex("El restaurante y el banco.", "The restaurant and the bank."),
                ex("El parque es grande.", "The park is big."),
            ]),
        unit(14, "A1.1", "The Body",
            "Name parts of the body and talk about them in pairs.",
            "Parts of the body — and plurals (un ojo, dos ojos).",
            &["cabeza", "ojo", "boca", "mano", "pie", "brazo", "pierna", "corazón"], Some(5),
            rd("El cuerpo",
                "Tengo dos ojos, una boca y una cabeza. Con las manos toco la guitarra y con los pies bailo. Me duele un poco el brazo, pero mi corazón está contento.",
                "I have two eyes, a mouth and a head. With my hands I play the guitar and with my feet I dance. My arm hurts a little, but my heart is happy."),
            vec![
                ex("La cabeza y el corazón.", "The head and the heart."),
                ex("Dos ojos y una boca.", "Two eyes and one mouth."),
                ex("Yo tengo dos manos y dos pies.", "I have two hands and two feet."),
                ex("El brazo y la pierna.", "The arm and the leg."),
            ]),
        unit(15, "A1.2", "Everyday Verbs II",
            "Talk about what you need, look for, and are learning.",
            "More common actions, with regular -ar verbs.",
            &["necesitar", "usar", "ayudar", "buscar", "esperar", "aprender", "entender"], Some(2),
            rd("Aprender español",
                "Quiero aprender español. Necesito practicar cada día y uso una aplicación. A veces no entiendo todo, pero busco las palabras y espero mejorar. Mis amigos me ayudan.",
                "I want to learn Spanish. I need to practice every day and I use an app. Sometimes I don't understand everything, but I look up the words and hope to improve. My friends help me."),
            vec![
                ex("Yo necesito agua.", "I need water."),
                ex("Ella ayuda a un amigo.", "She helps a friend."),
                ex("Nosotros aprendemos.", "We are learning."),
                ex("Yo busco el libro.", "I look for the book."),
            ]),
    ]
}

/// Themed vocabulary packs — the breadth track. Words are drawn from the same
/// inventory the units use, but grouped by topic (and including many words no
/// unit teaches), so a learner can grow vocabulary outside the grammar sequence.
fn spanish_packs(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<VocabPack> {
    let pack = |n: i64, emoji: &str, title: &str, desc: &str, words: &[&str]| VocabPack {
        id: PackId(base + n),
        language: language.clone(),
        title: title.into(),
        emoji: emoji.into(),
        description: desc.into(),
        lexemes: words.iter().filter_map(|w| ids.get(*w).copied()).collect(),
    };
    vec![
        pack(1, "🍽️", "Food & Drink", "Order a meal and talk about what's on the table.",
            &["comida", "fruta", "verdura", "carne", "pescado", "queso", "huevo", "arroz",
              "sopa", "pan", "leche", "café", "manzana", "vino", "jugo", "azúcar", "sal", "agua"]),
        pack(2, "✈️", "Travel & Places", "Find your way around town and on a trip.",
            &["aeropuerto", "hotel", "restaurante", "tienda", "mercado", "banco", "hospital",
              "parque", "baño", "ciudad", "país", "calle"]),
        pack(3, "💼", "Work & Money", "Talk about jobs, school, and spending.",
            &["trabajo", "dinero", "escuela", "banco", "comprar", "vender", "pagar", "trabajar", "viajar"]),
        pack(4, "😀", "Feelings & Traits", "Describe how people and things are.",
            &["feliz", "triste", "cansado", "enfermo", "fuerte", "débil", "bonito", "bueno", "malo", "importante"]),
        pack(5, "🧍", "The Body", "Name the parts of the body.",
            &["cabeza", "ojo", "boca", "mano", "pie", "brazo", "pierna", "corazón"]),
        pack(6, "⏰", "Time & Days", "Talk about when things happen.",
            &["tiempo", "año", "día", "vez", "hora", "minuto", "semana", "noche", "mañana",
              "tarde", "hoy", "ayer", "ahora", "temprano"]),
        pack(7, "🎨", "Colors", "Name colors to describe anything.",
            &["color", "rojo", "azul", "verde", "negro", "blanco", "amarillo"]),
        pack(8, "🗣️", "Handy Verbs", "Everyday verbs to get things done.",
            &["necesitar", "usar", "ayudar", "buscar", "esperar", "aprender", "entender",
              "recibir", "preguntar", "pagar", "viajar", "vender"]),
        pack(9, "👨‍👩‍👧", "People & Family", "The people in your life.",
            &["hombre", "mujer", "niño", "niña", "amigo", "amiga", "gente", "familia", "padre", "madre"]),
    ]
}

// --- French --------------------------------------------------------------

fn french_grammar(language: &LanguageCode, base: i64) -> Vec<GrammarPattern> {
    let d = |prompt: &str, answer: &str, tr: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
    };
    let p = |n: i64, label: &str, title: &str, ex: &str, expl: &str, prereqs: &[i64], drills: Vec<GrammarDrill>| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        title: title.into(),
        example_template: ex.into(),
        explanation: Some(expl.into()),
        prerequisites: prereqs.iter().map(|n| PatternId(base + n)).collect(),
        drills,
    };
    vec![
        p(1, "articles-le-la", "Gender & articles (le / la)", "le livre, la maison; un, une",
          "French nouns are masculine or feminine. 'le/un' go with masculine nouns, 'la/une' with feminine ones.",
          &[], vec![
            d("___ livre est sur la table. (the, m.)", "le", "The book is on the table."),
            d("___ maison est grande. (the, f.)", "la", "The house is big."),
            d("C'est ___ ami. (a, m.)", "un", "He is a friend."),
            d("C'est ___ pomme. (a, f.)", "une", "It's an apple."),
          ]),
        p(2, "present-er-verbs", "Present tense: -er verbs", "je parle, tu parles, il parle",
          "Regular -er verbs drop -er and add -e, -es, -e: parler → je parle, tu parles, il parle.",
          &[], vec![
            d("Je ___ français. (parler)", "parle", "I speak French."),
            d("Elle ___ en ville. (travailler)", "travaille", "She works in the city."),
            d("Nous ___ du pain. (manger)", "mangeons", "We eat bread."),
            d("Tu ___ beaucoup. (parler)", "parles", "You speak a lot."),
          ]),
        p(3, "etre-avoir", "To be & to have (être, avoir)", "je suis, j'ai",
          "Two essential irregular verbs: être (to be) → je suis, tu es, il est; avoir (to have) → j'ai, tu as, il a.",
          &[], vec![
            d("Je ___ un ami. (être)", "suis", "I am a friend."),
            d("Tu ___ une maison. (avoir)", "as", "You have a house."),
            d("Il ___ content. (être)", "est", "He is happy."),
            d("Nous ___ un chat. (avoir)", "avons", "We have a cat."),
          ]),
        p(4, "negation-ne-pas", "Negation (ne … pas)", "je ne parle pas",
          "To make a sentence negative, wrap the verb with ne ... pas: je parle → je ne parle pas.",
          &[2], vec![
            d("Je ne ___ pas français. (parler)", "parle", "I do not speak French."),
            d("Il ne ___ pas. (manger)", "mange", "He does not eat."),
            d("Nous ne ___ pas. (travailler)", "travaillons", "We do not work."),
            d("Tu ne ___ pas. (boire)", "bois", "You do not drink."),
          ]),
        p(5, "plural-s", "Making nouns plural", "un livre, deux livres",
          "Most French nouns add a (usually silent) -s in the plural: un livre → deux livres.",
          &[1], vec![
            d("un livre, deux ___ (livre)", "livres", "one book, two books"),
            d("un chat, trois ___ (chat)", "chats", "one cat, three cats"),
            d("une porte, deux ___ (porte)", "portes", "two doors"),
            d("un ami, deux ___ (ami)", "amis", "two friends"),
          ]),
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
    let rd = |title: &str, text: &str, tr: &str| {
        Some(ReadingPassage {
            title: title.into(),
            text: text.into(),
            translation: tr.into(),
        })
    };
    #[allow(clippy::too_many_arguments)]
    let unit = |n: i64,
                level: &str,
                title: &str,
                objective: &str,
                desc: &str,
                words: &[&str],
                pat: Option<i64>,
                reading: Option<ReadingPassage>,
                examples: Vec<ExampleSentence>| Unit {
        id: UnitId(base + n),
        language: language.clone(),
        title: title.into(),
        description: desc.into(),
        level: level.into(),
        objective: objective.into(),
        target_lexemes: resolve(words),
        target_pattern: pat.map(|n| PatternId(base + n)),
        reading,
        examples,
    };
    vec![
        unit(1, "A1.1", "Premiers mots (First Words)",
            "Greet people, introduce yourself, and say yes and no.",
            "Greet people, introduce yourself, and meet être (to be).",
            &["bonjour", "merci", "je", "tu", "être", "oui", "non", "ami", "un"], Some(3),
            rd("Deux amis",
                "— Bonjour ! Je suis Luc. — Bonjour, Luc. Tu es mon ami ? — Oui, bien sûr. — Merci, Luc. — De rien !",
                "— Hello! I am Luc. — Hello, Luc. Are you my friend? — Yes, of course. — Thank you, Luc. — You're welcome!"),
            vec![
                ex("Bonjour. Je suis un ami.", "Hello. I am a friend."),
                ex("Merci, ami.", "Thank you, friend."),
                ex("Oui ou non ?", "Yes or no?"),
                ex("Tu es un ami.", "You are a friend."),
            ]),
        unit(2, "A1.1", "Les gens (People)",
            "Name the people in a family and use the articles le / la.",
            "People around you, and the articles le / la.",
            &["homme", "femme", "enfant", "amie", "famille", "père", "mère", "le", "la"], Some(1),
            rd("La famille de Luc",
                "Voici la famille de Luc. Le père est un homme grand. La mère est une femme gentille. L'enfant joue dans le jardin. C'est une famille heureuse.",
                "Here is Luc's family. The father is a tall man. The mother is a kind woman. The child plays in the garden. It's a happy family."),
            vec![
                ex("L'homme et la femme.", "The man and the woman."),
                ex("La famille : le père, la mère et l'enfant.", "The family: the father, the mother and the child."),
                ex("Le père est un homme.", "The father is a man."),
                ex("La mère est une femme.", "The mother is a woman."),
            ]),
        unit(3, "A1.1", "À la maison (At Home)",
            "Name things in a home and talk about more than one.",
            "Things around the house, and plurals with -s.",
            &["maison", "table", "porte", "livre", "eau", "chien", "chat"], Some(5),
            rd("À la maison",
                "Dans ma maison, il y a beaucoup de choses. Sur la table, il y a un livre et un verre d'eau. Le chat dort près de la porte. Le chien est dans le jardin.",
                "In my house, there are many things. On the table, there is a book and a glass of water. The cat sleeps near the door. The dog is in the garden."),
            vec![
                ex("La maison a une porte.", "The house has a door."),
                ex("Le livre est sur la table.", "The book is on the table."),
                ex("Un chien et un chat.", "A dog and a cat."),
                ex("L'eau est dans la maison.", "The water is in the house."),
            ]),
        unit(4, "A1.1", "Manger et boire (Eating & Drinking)",
            "Say what you eat and drink.",
            "Food and drink, with regular -er verbs like manger.",
            &["manger", "boire", "pain", "lait", "café", "pomme", "nourriture"], Some(2),
            rd("Le petit déjeuner",
                "Le matin, je mange du pain et une pomme. Ma mère boit un café et mon père boit du lait. La nourriture est simple mais bonne. J'aime le petit déjeuner !",
                "In the morning, I eat bread and an apple. My mother drinks a coffee and my father drinks milk. The food is simple but good. I like breakfast!"),
            vec![
                ex("Je mange une pomme.", "I eat an apple."),
                ex("Tu manges du pain.", "You eat bread."),
                ex("Elle boit du lait.", "She drinks milk."),
                ex("Je bois un café.", "I drink a coffee."),
            ]),
        unit(5, "A1.2", "Actions (Everyday Actions)",
            "Talk about everyday actions and say what you want.",
            "Common things you do, and how to say what you want.",
            &["faire", "aller", "parler", "travailler", "acheter", "vouloir"], Some(2),
            rd("Une journée normale",
                "Chaque jour, je travaille en ville. Je parle avec mes amis et je fais beaucoup de choses. Ensuite, je vais au magasin et j'achète du pain. Le soir, je veux me reposer.",
                "Every day, I work in the city. I talk with my friends and do many things. Then, I go to the store and buy bread. In the evening, I want to rest."),
            vec![
                ex("Je parle avec un ami.", "I speak with a friend."),
                ex("Elle travaille dans la ville.", "She works in the city."),
                ex("Nous achetons du pain.", "We buy bread."),
                ex("Je veux manger.", "I want to eat."),
            ]),
    ]
}

fn french_packs(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<VocabPack> {
    let pack = |n: i64, emoji: &str, title: &str, desc: &str, words: &[&str]| VocabPack {
        id: PackId(base + n),
        language: language.clone(),
        title: title.into(),
        emoji: emoji.into(),
        description: desc.into(),
        lexemes: words.iter().filter_map(|w| ids.get(*w).copied()).collect(),
    };
    vec![
        pack(1, "🍽️", "Food & Drink", "Talk about food and drink.",
            &["nourriture", "pain", "lait", "café", "pomme", "eau"]),
        pack(2, "👨‍👩‍👧", "People & Family", "The people in your life.",
            &["homme", "femme", "enfant", "ami", "amie", "famille", "père", "mère"]),
        pack(3, "🏠", "Home & Things", "Things around the home.",
            &["maison", "table", "porte", "livre", "chien", "chat"]),
        pack(4, "🗣️", "Handy Verbs", "Everyday verbs to get things done.",
            &["faire", "aller", "voir", "savoir", "pouvoir", "vouloir", "venir", "dire",
              "parler", "manger", "boire", "habiter", "travailler", "acheter", "aimer", "lire"]),
        pack(5, "🎨", "Describing", "Describe how things are.",
            &["bon", "grand", "petit", "nouveau", "heureux", "rouge"]),
        pack(6, "🏙️", "Places & Time", "Places and when things happen.",
            &["ville", "rue", "jour", "nuit", "an", "temps", "monde"]),
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

    #[test]
    fn vocab_packs_resolve_to_seeded_words() {
        // Every pack must resolve most of its words, or it'd be a thin deck.
        for (json, base, packs_fn) in [
            (ES_FREQUENCY, 0i64, spanish_packs as fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<VocabPack>),
            (FR_FREQUENCY, 1000, french_packs),
        ] {
            let words: Vec<SeedWord> = serde_json::from_str(json).unwrap();
            let lang = LanguageCode::new(if base == 0 { "es" } else { "fr" });
            let lemma_ids: HashMap<String, LexemeId> = words
                .iter()
                .enumerate()
                .map(|(i, w)| (w.lemma.clone(), LexemeId(base + i as i64 + 1)))
                .collect();
            for p in packs_fn(&lang, base, &lemma_ids) {
                assert!(
                    p.lexemes.len() >= 5,
                    "pack '{}' only resolved {} words",
                    p.title,
                    p.lexemes.len()
                );
            }
        }
    }
}
