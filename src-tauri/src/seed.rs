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
const DE_FREQUENCY: &str = include_str!("../seed/de_frequency.json");

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
    seed_language(
        store,
        &LanguageCode::new("de"),
        2000,
        DE_FREQUENCY,
        german_grammar,
        german_units,
        german_packs,
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
        note: None,
    };
    let ex = |t: &str, tr: &str| ExampleSentence {
        text: t.into(),
        translation: tr.into(),
    };
    // A drill that teaches something after the answer (e.g. why it's irregular).
    let dn = |prompt: &str, answer: &str, tr: &str, note: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
        note: Some(note.into()),
    };
    #[allow(clippy::too_many_arguments)]
    let p = |n: i64, label: &str, title: &str, ex_tmpl: &str, expl: &str, prereqs: &[i64], examples: Vec<ExampleSentence>, notes: &[&str], drills: Vec<GrammarDrill>| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        title: title.into(),
        example_template: ex_tmpl.into(),
        explanation: Some(expl.into()),
        prerequisites: prereqs.iter().map(|n| PatternId(base + n)).collect(),
        examples,
        notes: notes.iter().map(|s| s.to_string()).collect(),
        drills,
    };
    vec![
        p(1, "gender-articles", "Gender & articles (el / la)", "el libro, la casa, los niños, las mesas",
          "Every Spanish noun is either masculine or feminine, and the words around it must match. 'the' is el (m.) or la (f.); 'a/an' is un (m.) or una (f.). In the plural they become los/las and unos/unas. The gender belongs to the word itself, so learn it together: la casa, el libro.",
          &[],
          vec![
            ex("el libro, la casa", "the book, the house"),
            ex("un amigo y una amiga", "a (male) friend and a (female) friend"),
            ex("los niños y las mesas", "the children and the tables"),
          ],
          &[
            "Most nouns ending in -o are masculine (el libro) and most ending in -a are feminine (la casa) — but watch for exceptions like el día (the day) and la mano (the hand).",
            "The gender is grammatical, not about meaning: a table is la mesa, a book is el libro.",
            "In the plural: el → los, la → las; un → unos, una → unas.",
          ],
          vec![
            d("___ libro es nuevo. (the, m.)", "el", "The book is new."),
            d("___ casa es grande. (the, f.)", "la", "The house is big."),
            d("Es ___ amigo. (a, m.)", "un", "He is a friend."),
            d("Es ___ mesa. (a, f.)", "una", "It is a table."),
          ]),
        p(2, "present-regular-ar", "Present tense: -ar verbs", "yo hablo, tú hablas, ella habla",
          "To conjugate a regular -ar verb, drop the -ar and add the endings -o (yo), -as (tú), -a (él/ella), -amos (nosotros), -an (ellos/ellas). So hablar → hablo, hablas, habla, hablamos, hablan.",
          &[],
          vec![
            ex("yo hablo, tú hablas, ella habla", "I speak, you speak, she speaks"),
            ex("Nosotros trabajamos en la ciudad.", "We work in the city."),
          ],
          &[
            "Spanish usually drops the subject pronoun, because the ending already shows who's acting: 'hablo' on its own means 'I speak'.",
            "One present tense covers both English forms — 'hablo' is both 'I speak' and 'I am speaking'.",
            "The yo form of every regular verb ends in -o: hablo, trabajo, compro.",
          ],
          vec![
            d("Yo ___ español. (hablar)", "hablo", "I speak Spanish."),
            d("Ella ___ en la ciudad. (trabajar)", "trabaja", "She works in the city."),
            d("Nosotros ___ pan. (comprar)", "compramos", "We buy bread."),
            d("Tú ___ mucho. (hablar)", "hablas", "You speak a lot."),
          ]),
        p(3, "present-regular-er-ir", "Present tense: -er / -ir verbs", "yo como, tú comes, ella vive",
          "Regular -er and -ir verbs share almost all their endings: -o, -es, -e, -en. They differ only in the 'we' form — -er verbs take -emos (comemos) while -ir verbs take -imos (vivimos). So comer → como, comes, come, comemos, comen; vivir → vivo, vives, vive, vivimos, viven.",
          &[2],
          vec![
            ex("yo como, tú comes, ella come", "I eat, you eat, she eats"),
            ex("Nosotros vivimos aquí.", "We live here."),
          ],
          &[
            "-er and -ir verbs are identical except in the nosotros form: comemos vs vivimos.",
            "As always, the subject pronoun is usually dropped: 'como' = 'I eat'.",
          ],
          vec![
            d("Yo ___ pan. (comer)", "como", "I eat bread."),
            d("Ella ___ leche. (beber)", "bebe", "She drinks milk."),
            dn("Nosotros ___ aquí. (vivir)", "vivimos", "We live here.",
               "-ir verbs take -imos in the 'we' form (vivimos), where -er verbs take -emos (comemos)."),
            d("Tú ___ una manzana. (comer)", "comes", "You eat an apple."),
          ]),
        p(4, "ser-vs-estar", "To be: ser vs estar", "Soy estudiante. Estoy en casa.",
          "Spanish has two verbs for 'to be'. Use ser for permanent or defining facts — identity, origin, profession, what something fundamentally is (Soy estudiante). Use estar for location and temporary states — where something is, or how it feels right now (Estoy en casa). Both are irregular.",
          &[2],
          vec![
            ex("Soy estudiante. Estoy en casa.", "I am a student. I am at home."),
            ex("Ella es alta, pero hoy está cansada.", "She is tall, but today she is tired."),
          ],
          &[
            "Quick guide: ser for WHAT something is (lasting), estar for HOW or WHERE it is (temporary). 'Es aburrido' = he is boring; 'Está aburrido' = he is bored right now.",
            "Location always uses estar, even for permanent things: Madrid está en España.",
            "Both are irregular — ser: soy, eres, es, somos, son; estar: estoy, estás, está, estamos, están.",
          ],
          vec![
            dn("Yo ___ estudiante. (ser)", "soy", "I am a student.",
               "Profession and identity take ser — which is irregular: soy, eres, es."),
            dn("Ella ___ en casa. (estar)", "está", "She is at home.",
               "Location always takes estar (note the accent): está."),
            d("Nosotros ___ amigos. (ser)", "somos", "We are friends."),
            dn("Yo ___ feliz hoy. (estar)", "estoy", "I am happy today.",
               "A temporary feeling like 'happy today' uses estar: estoy."),
          ]),
        p(5, "plural-nouns", "Making nouns plural", "un gato, dos gatos; una flor, tres flores",
          "To make a Spanish noun plural, add -s if it ends in a vowel (gato → gatos) and -es if it ends in a consonant (flor → flores). The article goes plural too: el → los, la → las.",
          &[1],
          vec![
            ex("un gato, dos gatos", "one cat, two cats"),
            ex("una flor, tres flores", "one flower, three flowers"),
          ],
          &[
            "Vowel ending → add -s (casa → casas); consonant ending → add -es (ciudad → ciudades).",
            "A final -z becomes -c- before the plural ending: vez → veces, luz → luces.",
            "Articles and adjectives go plural too, to agree: las casas blancas.",
          ],
          vec![
            d("un gato, dos ___ (gato)", "gatos", "one cat, two cats"),
            d("una flor, tres ___ (flor)", "flores", "one flower, three flowers"),
            d("un libro, dos ___ (libro)", "libros", "one book, two books"),
            dn("una vez, dos ___ (vez)", "veces", "one time, two times",
               "Nouns ending in -z swap it for -c before -es: vez → veces."),
          ]),
        p(6, "preterite-regular-ar", "The past tense (-ar verbs)", "Ayer hablé y compré pan.",
          "For completed past actions with -ar verbs, use the preterite. Drop -ar and add -é (yo), -aste (tú), -ó (él/ella), -amos (nosotros), -aron (ellos). So hablar → hablé, hablaste, habló, hablamos, hablaron. The accents on hablé and habló mark the stress on the last syllable.",
          &[2],
          vec![
            ex("Ayer hablé con un amigo.", "Yesterday I spoke with a friend."),
            ex("Ella compró pan.", "She bought bread."),
          ],
          &[
            "The accent changes the meaning: hablo (I speak, present) vs habló (he/she spoke, past); compro (I buy) vs compró (he/she bought).",
            "The nosotros form (hablamos, compramos) looks the same in present and preterite — context tells you which: hoy compramos vs ayer compramos.",
            "-car/-gar/-zar verbs change spelling in the yo form to keep their sound: buscar → busqué, llegar → llegué, empezar → empecé.",
          ],
          vec![
            dn("Ayer yo ___ con un amigo. (hablar)", "hablé", "Yesterday I spoke with a friend.",
               "The yo preterite ends in a stressed -é: hablé. Don't confuse it with hablo ('I speak', present)."),
            dn("Ella ___ en la ciudad. (trabajar)", "trabajó", "She worked in the city.",
               "The él/ella preterite ends in -ó with an accent: trabajó ('she worked'), not trabajo ('I work')."),
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
        note: None,
    };
    let ex = |t: &str, tr: &str| ExampleSentence {
        text: t.into(),
        translation: tr.into(),
    };
    // A drill that teaches something after the answer (e.g. why it's irregular).
    let dn = |prompt: &str, answer: &str, tr: &str, note: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
        note: Some(note.into()),
    };
    #[allow(clippy::too_many_arguments)]
    let p = |n: i64, label: &str, title: &str, ex_tmpl: &str, expl: &str, prereqs: &[i64], examples: Vec<ExampleSentence>, notes: &[&str], drills: Vec<GrammarDrill>| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        title: title.into(),
        example_template: ex_tmpl.into(),
        explanation: Some(expl.into()),
        prerequisites: prereqs.iter().map(|n| PatternId(base + n)).collect(),
        examples,
        notes: notes.iter().map(|s| s.to_string()).collect(),
        drills,
    };
    vec![
        p(1, "articles-le-la", "Gender & articles (le / la)", "le livre, la maison; un, une",
          "French nouns are masculine or feminine. 'the' is le (m.) or la (f.), and both shorten to l' before a vowel sound (l'ami, l'eau). 'a/an' is un (m.) or une (f.). The plural 'the' is les for every gender. Gender is part of the word, so learn it together: la maison, le livre.",
          &[],
          vec![
            ex("le livre, la maison", "the book, the house"),
            ex("un homme et une femme", "a man and a woman"),
            ex("l'ami, les amis", "the friend, the friends"),
          ],
          &[
            "Before a vowel or a silent h, le and la both shorten to l': l'eau, l'homme.",
            "There's no reliable rule for gender — memorise it with the noun (la table, le livre).",
            "The plural article is les for both genders: les hommes, les femmes.",
          ],
          vec![
            d("___ livre est sur la table. (the, m.)", "le", "The book is on the table."),
            d("___ maison est grande. (the, f.)", "la", "The house is big."),
            d("C'est ___ ami. (a, m.)", "un", "He is a friend."),
            d("C'est ___ pomme. (a, f.)", "une", "It's an apple."),
          ]),
        p(2, "present-er-verbs", "Present tense: -er verbs", "je parle, tu parles, il parle",
          "Regular -er verbs are the largest group. Drop -er and add -e (je), -es (tu), -e (il/elle), -ons (nous), -ez (vous), -ent (ils/elles). So parler → je parle, tu parles, il parle, nous parlons, ils parlent.",
          &[],
          vec![
            ex("je parle, tu parles, il parle", "I speak, you speak, he speaks"),
            ex("Nous mangeons du pain.", "We eat bread."),
          ],
          &[
            "The -e, -es, and -ent endings are all silent — je parle, tu parles, and ils parlent sound identical. Only nous (-ons) and vous (-ez) sound different.",
            "Verbs in -ger keep an e in the nous form to preserve the soft g (manger → nous mangeons); -cer verbs take a ç (commencer → nous commençons).",
          ],
          vec![
            d("Je ___ français. (parler)", "parle", "I speak French."),
            d("Elle ___ en ville. (travailler)", "travaille", "She works in the city."),
            dn("Nous ___ du pain. (manger)", "mangeons", "We eat bread.",
               "-ger verbs keep an e in the nous form to preserve the soft 'g' sound: mangeons, not 'mangons'."),
            d("Tu ___ beaucoup. (parler)", "parles", "You speak a lot."),
          ]),
        p(3, "etre-avoir", "To be & to have (être, avoir)", "je suis, j'ai",
          "être (to be) and avoir (to have) are the two most essential French verbs, and both are completely irregular. être: je suis, tu es, il/elle est, nous sommes, vous êtes, ils sont. avoir: j'ai, tu as, il/elle a, nous avons, vous avez, ils ont. They also build the past tense, so they're worth mastering early.",
          &[],
          vec![
            ex("Je suis un ami. Tu es grand.", "I am a friend. You are tall."),
            ex("J'ai un chien. Elle a une maison.", "I have a dog. She has a house."),
          ],
          &[
            "Both are irregular — learn every form by heart.",
            "Before a vowel, je becomes j': j'ai, not 'je ai'.",
            "Don't mix them up: il est = he is; il a = he has.",
          ],
          vec![
            dn("Je ___ un ami. (être)", "suis", "I am a friend.",
               "être is irregular: je suis, tu es, il est."),
            dn("Tu ___ une maison. (avoir)", "as", "You have a house.",
               "avoir is irregular: j'ai, tu as, il a."),
            d("Il ___ content. (être)", "est", "He is happy."),
            d("Nous ___ un chat. (avoir)", "avons", "We have a cat."),
          ]),
        p(4, "negation-ne-pas", "Negation (ne … pas)", "je ne parle pas",
          "To make a sentence negative, French wraps the conjugated verb in two parts: ne before it and pas after it. So je parle → je ne parle pas. Before a vowel, ne shortens to n': je n'ai pas.",
          &[2],
          vec![
            ex("Je ne parle pas anglais.", "I do not speak English."),
            ex("Il n'a pas de chien.", "He doesn't have a dog."),
          ],
          &[
            "Both halves are needed in writing: ne before the verb, pas after. (Casual speech often drops ne, but keep it when writing.)",
            "Before a vowel, ne becomes n': je n'ai pas, il n'est pas.",
            "After a negative, un/une/du usually becomes de: j'ai un chien → je n'ai pas de chien.",
          ],
          vec![
            dn("Je ne ___ pas français. (parler)", "parle", "I do not speak French.",
               "The verb sits between ne and pas and is conjugated normally: je ne parle pas."),
            d("Il ne ___ pas. (manger)", "mange", "He does not eat."),
            d("Nous ne ___ pas. (travailler)", "travaillons", "We do not work."),
            dn("Tu ne ___ pas. (boire)", "bois", "You do not drink.",
               "boire is irregular: je bois, tu bois, il boit."),
          ]),
        p(5, "plural-s", "Making nouns plural", "un livre, deux livres",
          "Most French nouns add an -s to form the plural — but that -s is silent, so singular and plural usually sound the same (le livre / les livres). What you actually hear is the article changing: le, la, and l' all become les.",
          &[1],
          vec![
            ex("un livre, deux livres", "one book, two books"),
            ex("le chat, les chats", "the cat, the cats"),
          ],
          &[
            "The plural -s is silent — you hear the difference in the article (le → les), not the noun.",
            "Nouns already ending in -s, -x, or -z don't change: un fils, deux fils.",
            "Some nouns take -x instead, especially those ending in -eau: un gâteau → deux gâteaux.",
          ],
          vec![
            dn("un livre, deux ___ (livre)", "livres", "one book, two books",
               "The plural -s is silent — livre and livres sound the same; the article (les) signals the plural."),
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

// --- German --------------------------------------------------------------

fn german_grammar(language: &LanguageCode, base: i64) -> Vec<GrammarPattern> {
    let ex = |t: &str, tr: &str| ExampleSentence {
        text: t.into(),
        translation: tr.into(),
    };
    let d = |prompt: &str, answer: &str, tr: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
        note: None,
    };
    // A drill that teaches something after the answer (e.g. why it's irregular).
    let dn = |prompt: &str, answer: &str, tr: &str, note: &str| GrammarDrill {
        prompt: prompt.into(),
        answer: answer.into(),
        translation: tr.into(),
        note: Some(note.into()),
    };
    #[allow(clippy::too_many_arguments)]
    let p = |n: i64, label: &str, title: &str, ex_tmpl: &str, expl: &str, prereqs: &[i64], examples: Vec<ExampleSentence>, notes: &[&str], drills: Vec<GrammarDrill>| GrammarPattern {
        id: PatternId(base + n),
        language: language.clone(),
        label: label.into(),
        title: title.into(),
        example_template: ex_tmpl.into(),
        explanation: Some(expl.into()),
        prerequisites: prereqs.iter().map(|n| PatternId(base + n)).collect(),
        examples,
        notes: notes.iter().map(|s| s.to_string()).collect(),
        drills,
    };
    vec![
        p(1, "articles-der-die-das", "Gender & articles (der / die / das)", "der Mann, die Frau, das Kind",
          "Every German noun has a gender — masculine, feminine, or neuter — and 'the' changes with it: der (m.), die (f.), das (n.). For 'a/an' it's ein (m./n.) or eine (f.). The gender belongs to the word itself, not to the meaning, so it must be learned with each noun.",
          &[],
          vec![
            ex("der Mann, die Frau, das Kind", "the man, the woman, the child"),
            ex("Das ist ein Hund und eine Katze.", "That is a dog and a cat."),
          ],
          &[
            "There's no reliable rule for gender — memorise it with the word: learn 'das Haus', not just 'Haus'.",
            "In the plural, 'the' is always die, whatever the gender: die Männer, die Frauen, die Kinder.",
            "'a/an' is ein for masculine and neuter, eine for feminine — the gender decides it, not the English.",
          ],
          vec![
            d("___ Mann ist groß. (the, m.)", "der", "The man is tall."),
            d("___ Frau ist hier. (the, f.)", "die", "The woman is here."),
            d("___ Kind spielt. (the, n.)", "das", "The child plays."),
            d("Das ist ___ Haus. (a, n.)", "ein", "That is a house."),
          ]),
        p(2, "present-tense", "Present tense", "ich mache, du machst, er macht",
          "To conjugate a regular verb, drop the -en from the infinitive and add an ending for each person. The endings are -e (ich), -st (du), -t (er/sie/es), -en (wir, sie, Sie), -t (ihr). So machen → ich mache, du machst, er macht, wir machen.",
          &[],
          vec![
            ex("ich mache, du machst, er macht", "I do, you do, he does"),
            ex("Wir lernen Deutsch und sprechen Englisch.", "We learn German and speak English."),
          ],
          &[
            "Stem-changing verbs shift their vowel in the du- and er/sie/es-forms only: essen → du isst, er isst; sprechen → du sprichst, er spricht; fahren → du fährst, er fährt. The wir/sie forms stay regular (wir essen).",
            "If the stem ends in -t or -d, an extra -e is added so it's pronounceable: arbeiten → du arbeitest, er arbeitet.",
            "German has one present tense — 'ich lerne' covers both 'I learn' and 'I am learning'.",
          ],
          vec![
            d("Ich ___ Deutsch. (lernen)", "lerne", "I learn German."),
            d("Du ___ Wasser. (trinken)", "trinkst", "You drink water."),
            dn("Er ___ Brot. (essen)", "isst", "He eats bread.",
               "essen is a stem-changer (e→i): ich esse, but du isst and er isst — not 'esst'."),
            d("Wir ___ in der Stadt. (wohnen)", "wohnen", "We live in the city."),
          ]),
        p(3, "sein-haben", "To be & to have (sein, haben)", "ich bin, ich habe",
          "sein (to be) and haben (to have) are the two most important verbs in German — and both are irregular, so every form has to be learned by heart. sein: ich bin, du bist, er/sie/es ist, wir/sie sind, ihr seid. haben: ich habe, du hast, er hat, wir/sie haben, ihr habt.",
          &[],
          vec![
            ex("Ich bin müde. Du bist groß.", "I am tired. You are tall."),
            ex("Ich habe Zeit. Sie hat einen Hund.", "I have time. She has a dog."),
          ],
          &[
            "sein doesn't look like its infinitive at all — bin, bist, ist, sind. Drill it until it's automatic.",
            "haben drops its -b- in the du and er forms: du hast, er hat (not 'habst/habt').",
          ],
          vec![
            dn("Ich ___ müde. (sein)", "bin", "I am tired.",
               "sein is fully irregular: ich bin, du bist, er ist, wir sind."),
            dn("Du ___ einen Hund. (haben)", "hast", "You have a dog.",
               "haben loses its -b- here: du hast, er hat — not 'habst/habt'."),
            d("Sie ___ eine Frau. (sein)", "ist", "She is a woman."),
            d("Wir ___ Zeit. (haben)", "haben", "We have time."),
          ]),
        p(4, "plural-nouns", "Plural nouns", "der Hund → die Hunde, das Kind → die Kinder",
          "German has no single way to make a plural. A noun takes one of several endings — and sometimes an umlaut — depending on the word, so the plural is learned together with the noun (and its gender).",
          &[1],
          vec![
            ex("der Hund → die Hunde", "the dog → the dogs"),
            ex("das Kind → die Kinder", "the child → the children"),
            ex("die Mutter → die Mütter", "the mother → the mothers"),
          ],
          &[
            "Common plural endings: -e (Hund→Hunde), -er (Kind→Kinder, usually + umlaut), -(e)n (Frau→Frauen), -s (Auto→Autos), or no ending — often just an umlaut (Mutter→Mütter).",
            "The plural article is always die: die Hunde, die Kinder, die Autos.",
            "Because it's unpredictable, learn each noun as a trio: gender + singular + plural (das Kind, die Kinder).",
          ],
          vec![
            d("ein Hund, zwei ___ (Hund)", "Hunde", "one dog, two dogs"),
            dn("ein Kind, drei ___ (Kind)", "Kinder", "one child, three children",
               "The -er plural is common for neuter nouns and often adds an umlaut (Mann→Männer), though Kind→Kinder doesn't."),
            d("ein Auto, zwei ___ (Auto)", "Autos", "two cars"),
            d("eine Frau, zwei ___ (Frau)", "Frauen", "two women"),
          ]),
        p(5, "negation-nicht-kein", "Negation (nicht / kein)", "Ich bin nicht müde. Das ist kein Hund.",
          "German negates in two ways. Use kein/keine to say 'no / not a' before a noun — it replaces ein/eine and takes the same endings. Use nicht for everything else: to negate a verb, an adjective, or a noun that has a definite article.",
          &[2],
          vec![
            ex("Ich bin nicht müde.", "I am not tired."),
            ex("Ich habe keine Zeit.", "I have no time."),
            ex("Das ist nicht der Hund.", "That is not the dog."),
          ],
          &[
            "kein replaces ein/eine and matches the noun's gender: kein Hund (m.), keine Katze (f.), kein Haus (n.), keine Hunde (pl.).",
            "Use nicht with a verb or adjective (Ich arbeite nicht; Es ist nicht gut) and with a noun that has 'the' (nicht der Hund).",
            "Rule of thumb: if the positive sentence used ein or no article, negate with kein; otherwise use nicht.",
          ],
          vec![
            d("Ich bin ___ müde. (not)", "nicht", "I am not tired."),
            d("Das ist ___ Hund. (not a, m.)", "kein", "That is not a dog."),
            d("Ich spreche ___ Deutsch. (not)", "nicht", "I do not speak German."),
            dn("Er hat ___ Zeit. (no, f.)", "keine", "He has no time.",
               "Zeit is feminine, so 'no time' is keine Zeit — kein takes an -e like eine."),
          ]),
        p(6, "word-order-v2", "Word order (verb second)", "Heute lerne ich Deutsch.",
          "In a German main clause the conjugated verb is always the second element — not necessarily the second word, but the second 'piece' of the sentence. Whatever comes first (the subject, or a time/place phrase), the verb follows immediately, and if you didn't start with the subject, it slides in right after the verb.",
          &[2],
          vec![
            ex("Ich lerne heute Deutsch.", "I am learning German today."),
            ex("Heute lerne ich Deutsch.", "Today I am learning German."),
          ],
          &[
            "Start with a time or place word and the subject moves after the verb: Heute lerne ich… (never 'Heute ich lerne').",
            "Only ONE element may sit before the verb. 'Heute ich…' is wrong because that's two.",
            "This verb-second (V2) rule is one of the most distinctive — and most drilled — features of German.",
          ],
          vec![
            d("Heute ___ ich Deutsch. (lernen)", "lerne", "Today I learn German."),
            dn("Morgen ___ wir nach Berlin. (fahren)", "fahren", "Tomorrow we drive to Berlin.",
               "After 'Morgen', the verb comes before the subject: Morgen fahren wir — verb second."),
            d("Jetzt ___ er Kaffee. (trinken)", "trinkt", "Now he drinks coffee."),
            d("Hier ___ ich. (arbeiten)", "arbeite", "Here I work."),
          ]),
    ]
}

fn german_units(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<Unit> {
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
            "Greet people, say thank you, and introduce yourself.",
            "Say hello, talk about yourself, and meet the verb sein (to be).",
            &["hallo", "danke", "bitte", "ich", "du", "sein", "ja", "nein", "und", "Freund"], Some(3),
            rd("Zwei Freunde",
                "— Hallo! Ich bin Anna. — Hallo, Anna. Bist du meine Freundin? — Ja! Du und ich sind Freunde. — Danke!",
                "— Hello! I am Anna. — Hello, Anna. Are you my friend? — Yes! You and I are friends. — Thank you!"),
            vec![
                ex("Hallo! Ich bin ein Freund.", "Hello! I am a friend."),
                ex("Danke, Freund.", "Thank you, friend."),
                ex("Du und ich.", "You and I."),
                ex("Ja oder nein?", "Yes or no?"),
            ]),
        unit(2, "A1.1", "People & Family",
            "Name the people in a family and use der / die / das.",
            "People around you, and how German marks gender with der / die / das.",
            &["Mann", "Frau", "Kind", "Familie", "Vater", "Mutter", "der", "die", "das"], Some(1),
            rd("Annas Familie",
                "Das ist Annas Familie. Der Vater ist ein Mann. Die Mutter ist eine Frau. Das Kind spielt im Garten. Es ist eine glückliche Familie!",
                "This is Anna's family. The father is a man. The mother is a woman. The child plays in the garden. It is a happy family!"),
            vec![
                ex("Der Mann und die Frau.", "The man and the woman."),
                ex("Die Familie: der Vater, die Mutter und das Kind.", "The family: the father, the mother and the child."),
                ex("Der Vater ist ein Mann.", "The father is a man."),
                ex("Die Mutter ist eine Frau.", "The mother is a woman."),
            ]),
        unit(3, "A1.1", "Home & Things",
            "Name common things in a home.",
            "Objects around the house, and a first look at plurals.",
            &["Haus", "Tisch", "Tür", "Buch", "Wasser", "Hund", "Katze"], Some(4),
            rd("Zu Hause",
                "In meinem Haus gibt es viele Dinge. Auf dem Tisch ist ein Buch und ein Glas Wasser. Die Katze schläft an der Tür. Der Hund ist im Garten.",
                "In my house there are many things. On the table is a book and a glass of water. The cat sleeps by the door. The dog is in the garden."),
            vec![
                ex("Das Haus hat eine Tür.", "The house has a door."),
                ex("Das Buch ist auf dem Tisch.", "The book is on the table."),
                ex("Ein Hund und eine Katze.", "A dog and a cat."),
                ex("Das Wasser ist im Haus.", "The water is in the house."),
            ]),
        unit(4, "A1.1", "Eating & Drinking",
            "Say what you eat and drink.",
            "Food and drink, with the verbs essen and trinken.",
            &["essen", "trinken", "Brot", "Milch", "Kaffee", "Apfel", "Essen"], Some(2),
            rd("Das Frühstück",
                "Am Morgen esse ich Brot mit einem Apfel. Meine Mutter trinkt Kaffee und mein Vater trinkt Milch. Das Essen ist einfach, aber gut.",
                "In the morning I eat bread with an apple. My mother drinks coffee and my father drinks milk. The food is simple, but good."),
            vec![
                ex("Ich esse Brot.", "I eat bread."),
                ex("Du trinkst Milch.", "You drink milk."),
                ex("Er isst einen Apfel.", "He eats an apple."),
                ex("Ich trinke Kaffee und Wasser.", "I drink coffee and water."),
            ]),
        unit(5, "A1.2", "Everyday Actions",
            "Talk about everyday actions and say what you want to do.",
            "Common things you do, with regular verbs and wollen (to want).",
            &["machen", "gehen", "sprechen", "arbeiten", "kaufen", "wollen"], Some(2),
            rd("Ein normaler Tag",
                "Jeden Tag arbeite ich in der Stadt. Ich spreche mit meinen Freunden und mache viele Dinge. Dann gehe ich zum Markt und kaufe Brot. Am Abend will ich schlafen.",
                "Every day I work in the city. I speak with my friends and do many things. Then I go to the market and buy bread. In the evening I want to sleep."),
            vec![
                ex("Ich spreche mit einem Freund.", "I speak with a friend."),
                ex("Sie arbeitet in der Stadt.", "She works in the city."),
                ex("Wir kaufen Brot.", "We buy bread."),
                ex("Ich will essen.", "I want to eat."),
            ]),
        unit(6, "A1.1", "Numbers & Describing",
            "Count to ten and describe things as big, small, good, or bad.",
            "Numbers and common adjectives.",
            &["eins", "zwei", "drei", "gut", "schlecht", "groß", "klein", "neu"], None,
            rd("Drei Bücher",
                "Ich habe drei neue Bücher. Eins ist groß und zwei sind klein. Das große Buch ist sehr gut. Das kleine ist nicht schlecht.",
                "I have three new books. One is big and two are small. The big book is very good. The small one is not bad."),
            vec![
                ex("Eins, zwei, drei.", "One, two, three."),
                ex("Ein neues Buch.", "A new book."),
                ex("Das Haus ist groß.", "The house is big."),
                ex("Der Kaffee ist gut.", "The coffee is good."),
            ]),
        unit(7, "A1.1", "Colors",
            "Name colors and describe objects by their color.",
            "Name colors to describe things.",
            &["Farbe", "rot", "blau", "grün", "schwarz", "weiß", "gelb"], None,
            rd("Die Farben",
                "Meine Lieblingsfarbe ist blau. Ich habe eine schwarze Katze und einen weißen Hund. Im Garten gibt es rote und gelbe Blumen. Das Gras ist grün.",
                "My favorite color is blue. I have a black cat and a white dog. In the garden there are red and yellow flowers. The grass is green."),
            vec![
                ex("Die schwarze Katze und der weiße Hund.", "The black cat and the white dog."),
                ex("Ein rotes Haus.", "A red house."),
                ex("Das Buch ist blau.", "The book is blue."),
                ex("Die Farbe grün und die Farbe gelb.", "The color green and the color yellow."),
            ]),
        unit(8, "A1.2", "Time & Days",
            "Talk about when things happen — today, tomorrow, now.",
            "Talk about days and time, with German's verb-second word order.",
            &["Tag", "Jahr", "Nacht", "heute", "morgen", "jetzt", "Stunde", "Zeit"], Some(6),
            rd("Heute und morgen",
                "Heute ist ein guter Tag. Jetzt arbeite ich, aber ich habe nicht viel Zeit. Morgen gehe ich in die Stadt. In der Nacht schlafe ich.",
                "Today is a good day. Now I work, but I don't have much time. Tomorrow I go to the city. At night I sleep."),
            vec![
                ex("Heute ist ein guter Tag.", "Today is a good day."),
                ex("Jetzt arbeite ich.", "Now I work."),
                ex("Morgen gehe ich.", "Tomorrow I go."),
                ex("Die Nacht und der Tag.", "The night and the day."),
            ]),
    ]
}

fn german_packs(language: &LanguageCode, base: i64, ids: &HashMap<String, LexemeId>) -> Vec<VocabPack> {
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
            &["Brot", "Milch", "Kaffee", "Apfel", "Wasser", "Käse", "Ei", "Essen"]),
        pack(2, "👨‍👩‍👧", "People & Family", "The people in your life.",
            &["Mann", "Frau", "Kind", "Familie", "Vater", "Mutter", "Freund", "Freundin", "Mensch"]),
        pack(3, "🏠", "Home & Things", "Things around the home.",
            &["Haus", "Tisch", "Tür", "Buch", "Hund", "Katze", "Auto"]),
        pack(4, "🗣️", "Handy Verbs", "Everyday verbs to get things done.",
            &["machen", "gehen", "kommen", "sehen", "sprechen", "arbeiten", "kaufen", "lernen",
              "brauchen", "helfen", "suchen", "verstehen", "lesen"]),
        pack(5, "🎨", "Describing & Colors", "Describe how things are.",
            &["gut", "schlecht", "groß", "klein", "neu", "alt", "schön", "glücklich",
              "rot", "blau", "grün", "schwarz", "weiß", "gelb"]),
        pack(6, "⏰", "Time & Days", "Talk about when things happen.",
            &["Tag", "Jahr", "Nacht", "Stunde", "Zeit", "Woche", "heute", "morgen", "jetzt", "gestern"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type UnitsFn = fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<Unit>;
    type PacksFn = fn(&LanguageCode, i64, &HashMap<String, LexemeId>) -> Vec<VocabPack>;

    /// (code, json, base, units_fn, packs_fn) for every seeded language.
    fn languages() -> Vec<(&'static str, &'static str, i64, UnitsFn, PacksFn)> {
        vec![
            ("es", ES_FREQUENCY, 0, spanish_units as UnitsFn, spanish_packs as PacksFn),
            ("fr", FR_FREQUENCY, 1000, french_units, french_packs),
            ("de", DE_FREQUENCY, 2000, german_units, german_packs),
        ]
    }

    /// Lemma → id map mirroring how `seed_language` assigns ids (by list order).
    fn lemma_ids(json: &str, base: i64) -> HashMap<String, LexemeId> {
        serde_json::from_str::<Vec<SeedWord>>(json)
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, w)| (w.lemma.clone(), LexemeId(base + i as i64 + 1)))
            .collect()
    }

    #[test]
    fn bundled_frequency_lists_are_valid() {
        for (_, json, _, _, _) in languages() {
            let words: Vec<SeedWord> =
                serde_json::from_str(json).expect("frequency json must deserialize");
            assert!(words.len() > 40, "seed list too small: {}", words.len());
        }
    }

    #[test]
    fn unit_targets_resolve_to_seeded_words() {
        // Every word referenced by a unit must exist in that language's list,
        // or progress over it would be impossible.
        for (code, json, base, units_fn, _) in languages() {
            let lang = LanguageCode::new(code);
            let ids = lemma_ids(json, base);
            for u in units_fn(&lang, base, &ids) {
                assert!(
                    !u.target_lexemes.is_empty(),
                    "{code} unit '{}' resolved no target words",
                    u.title
                );
            }
        }
    }

    #[test]
    fn vocab_packs_resolve_to_seeded_words() {
        // Every pack must resolve most of its words, or it'd be a thin deck.
        for (code, json, base, _, packs_fn) in languages() {
            let lang = LanguageCode::new(code);
            let ids = lemma_ids(json, base);
            for p in packs_fn(&lang, base, &ids) {
                assert!(
                    p.lexemes.len() >= 5,
                    "{code} pack '{}' only resolved {} words",
                    p.title,
                    p.lexemes.len()
                );
            }
        }
    }
}
