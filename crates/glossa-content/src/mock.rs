//! Offline, deterministic generator. No network, no API key.
//!
//! It stitches the chosen new words together with a few known words so the
//! reading/review loop works end-to-end immediately. Output is intentionally
//! simple (and not always grammatical) — it exists to exercise the pipeline,
//! not to teach. Swap in [`crate::AnthropicContentGenerator`] for real content.

use async_trait::async_trait;

use glossa_core::{ContentRequest, GeneratedContent};

use crate::{ContentError, ContentGenerator, Result, SuggestedWord, VocabRequest};

pub struct MockContentGenerator;

#[async_trait]
impl ContentGenerator for MockContentGenerator {
    async fn suggest_vocab(&self, _request: &VocabRequest) -> Result<Vec<SuggestedWord>> {
        Err(ContentError::Config(
            "translating words needs the AI engine — set ANTHROPIC_API_KEY to enable it".into(),
        ))
    }

    async fn generate(&self, request: &ContentRequest) -> Result<GeneratedContent> {
        let known: Vec<String> = request
            .known_vocab
            .iter()
            .map(|l| l.lemma.clone())
            .collect();
        let new_words: Vec<String> = request
            .new_targets
            .iter()
            .map(|l| l.lemma.clone())
            .collect();

        if known.is_empty() && new_words.is_empty() {
            return Err(ContentError::NoContent);
        }

        // Look up a word's meaning from the request's vocab.
        let gloss_of = |lemma: &String| -> String {
            request
                .known_vocab
                .iter()
                .chain(request.new_targets.iter())
                .find(|l| &l.lemma == lemma)
                .and_then(|l| l.gloss.clone())
                .unwrap_or_else(|| lemma.clone())
        };

        // One short clause per new word, padded with up to two known words so
        // the new word appears in *some* context. Falls back to known-only.
        let mut es: Vec<String> = Vec::new();
        let mut en: Vec<String> = Vec::new();
        let mut known_used: Vec<String> = Vec::new();

        let pad = |i: usize| -> Vec<String> {
            known.iter().cycle().skip(i * 2).take(2).cloned().collect()
        };

        if new_words.is_empty() {
            let words: Vec<String> = known.iter().take(4).cloned().collect();
            known_used.extend(words.iter().cloned());
            en.push(join_sentence(&words.iter().map(gloss_of).collect::<Vec<_>>()));
            es.push(join_sentence(&words));
        } else {
            for (i, nw) in new_words.iter().enumerate() {
                let context = pad(i);
                let mut es_clause: Vec<String> = Vec::new();
                let mut en_clause: Vec<String> = Vec::new();
                if let Some(first) = context.first() {
                    es_clause.push(first.clone());
                    en_clause.push(gloss_of(first));
                    known_used.push(first.clone());
                }
                es_clause.push(nw.clone());
                en_clause.push(gloss_of(nw));
                if let Some(last) = context.get(1) {
                    es_clause.push(last.clone());
                    en_clause.push(gloss_of(last));
                    known_used.push(last.clone());
                }
                es.push(join_sentence(&es_clause));
                en.push(join_sentence(&en_clause));
            }
        }

        known_used.sort();
        known_used.dedup();

        Ok(GeneratedContent {
            text: es.join(" "),
            known_words_used: known_used,
            new_words_introduced: new_words,
            grammar_targeted: request.grammar_target.as_ref().map(|g| g.label.clone()),
            translation: Some(en.join(" ")),
        })
    }
}

/// Capitalize the first word and end with a period.
fn join_sentence(words: &[String]) -> String {
    let mut s = words.join(" ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s.push('.');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_core::{ContentKind, LanguageCode, LearnerId, Lexeme, LexemeId, PartOfSpeech};

    fn lex(id: i64, lemma: &str) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::spanish(),
            lemma: lemma.into(),
            pos: PartOfSpeech::Noun,
            frequency_rank: id as u32,
            gloss: Some(format!("gloss-{lemma}")),
        }
    }

    #[tokio::test]
    async fn mock_introduces_new_words_and_reports_them() {
        let req = ContentRequest {
            learner_id: LearnerId::new(),
            language: LanguageCode::spanish(),
            kind: ContentKind::Sentence,
            known_vocab: vec![lex(1, "yo"), lex(2, "comer")],
            new_targets: vec![lex(3, "pizza")],
            grammar_target: None,
            known_ratio: 0.95,
        };
        let out = MockContentGenerator.generate(&req).await.unwrap();
        assert!(out.text.to_lowercase().contains("pizza"));
        assert_eq!(out.new_words_introduced, vec!["pizza".to_string()]);
        assert!(!out.known_words_used.is_empty());
    }
}
