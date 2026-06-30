<script>
	// Shared Study → Quiz → Done flow, used by both built-in packs and user decks.
	// The parent supplies an already-loaded `lesson` ({ cards }) and a `loadQuiz`
	// function that returns the quiz items; everything else is generic.
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';
	import Exercise from '$lib/Exercise.svelte';

	let { lesson, loadQuiz, lang = 'es', onExit, exitLabel = 'Done →' } = $props();

	let phase = $state('study'); // 'study' | 'quiz' | 'done'
	let error = $state('');

	let cardIdx = $state(0);
	let flipped = $state(false);
	let card = $derived(cardIdx < lesson.cards.length ? lesson.cards[cardIdx] : null);

	let items = $state([]);
	let quizLoading = $state(false);
	let qIdx = $state(0);
	let answered = $state(false);
	let correctCount = $state(0);
	let streak = $state(0);
	let question = $derived(qIdx < items.length ? items[qIdx] : null);

	function flip() {
		flipped = !flipped;
		if (flipped && card) speak(card.lemma, lang);
	}
	function nextCard() {
		flipped = false;
		if (cardIdx < lesson.cards.length - 1) cardIdx += 1;
	}
	function prevCard() {
		flipped = false;
		if (cardIdx > 0) cardIdx -= 1;
	}

	async function startQuiz() {
		phase = 'quiz';
		quizLoading = true;
		qIdx = 0;
		answered = false;
		correctCount = 0;
		error = '';
		try {
			items = await loadQuiz();
		} catch (e) {
			error = String(e);
		} finally {
			quizLoading = false;
		}
	}

	async function onAnswer(correct) {
		answered = true;
		if (correct) correctCount += 1;
		try {
			const r = await api.recordExercise(question.lexeme_id, correct);
			streak = r.streak;
		} catch (e) {
			error = String(e);
		}
	}
	function nextQuestion() {
		answered = false;
		if (qIdx + 1 < items.length) qIdx += 1;
		else phase = 'done';
	}

	function restart() {
		phase = 'study';
		cardIdx = 0;
		flipped = false;
	}
</script>

{#if error}<div class="error" style="margin-bottom: 0.6rem;">{error}</div>{/if}

{#if lesson.cards.length === 0}
	<div class="card"><p class="muted">No words yet — add some first.</p></div>
{:else if phase === 'study'}
	<div class="card flashcard">
		<div class="fc-progress">Card {cardIdx + 1} of {lesson.cards.length}</div>
		{#if card}
			<div
				class="fc {flipped ? 'flipped' : ''}"
				role="button"
				tabindex="0"
				onclick={flip}
				onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && flip()}>
				{#if !flipped}
					<div class="fc-word">{card.lemma}</div>
					<div class="fc-hint">tap to reveal meaning</div>
				{:else}
					<div class="fc-gloss">{card.gloss ?? 'no meaning on file'}</div>
					<div class="fc-word small">{card.lemma}
						<button class="iconbtn" title="Listen" onclick={(e) => { e.stopPropagation(); speak(card.lemma, lang); }}>🔊</button>
					</div>
					{#if card.status !== 'unknown'}<div class="fc-hint">you've seen this before</div>{/if}
				{/if}
			</div>
		{/if}
		<div class="nav">
			<button class="link" onclick={prevCard} disabled={cardIdx === 0}>← Prev</button>
			{#if cardIdx < lesson.cards.length - 1}
				<button class="primary" onclick={nextCard}>Next →</button>
			{:else}
				<button class="primary" onclick={startQuiz}>Quiz me →</button>
			{/if}
		</div>
	</div>
	<div class="row" style="justify-content: center; margin-top: 0.8rem;">
		<button class="link" onclick={startQuiz}>Skip studying — quiz me now</button>
	</div>
{:else if phase === 'quiz'}
	{#if quizLoading}
		<p class="muted">Building your quiz…</p>
	{:else if question}
		<div class="card quiz">
			<div class="fc-progress">Question {qIdx + 1} of {items.length}</div>
			{#key qIdx}
				<Exercise item={question} {lang} {onAnswer} />
			{/key}
			{#if answered}
				<div class="row" style="justify-content: flex-end; margin-top: 1.2rem;">
					<button class="primary" onclick={nextQuestion}>
						{qIdx + 1 < items.length ? 'Next →' : 'Finish'}
					</button>
				</div>
			{/if}
		</div>
	{:else}
		<div class="card"><p class="muted">No quizzable words here yet — add meanings to your words.</p>
			<button class="primary" onclick={onExit}>{exitLabel}</button>
		</div>
	{/if}
{:else if phase === 'done'}
	<div class="card celebrate">
		<div class="emoji-big">{correctCount === items.length ? '🏆' : '✅'}</div>
		<h2>Nice work!</h2>
		<p>You got <strong>{correctCount}</strong> of <strong>{items.length}</strong> right.</p>
		{#if streak > 0}<p class="streak-big">🔥 {streak}-day streak</p>{/if}
		<div class="row" style="justify-content: center; gap: 0.7rem;">
			<button onclick={restart}>Study again</button>
			<button class="primary" onclick={onExit}>{exitLabel}</button>
		</div>
	</div>
{/if}

<style>
	.fc-progress {
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
	}
	.flashcard {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.fc {
		width: 100%;
		min-height: 11rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.6rem;
		border: 1px solid var(--border);
		border-radius: 14px;
		background: var(--panel-2);
		cursor: pointer;
		padding: 1.5rem;
	}
	.fc.flipped {
		border-color: var(--accent);
	}
	.fc-word {
		font-size: 2.4rem;
		font-weight: 700;
	}
	.fc-word.small {
		font-size: 1.3rem;
		font-weight: 600;
		color: var(--muted);
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.fc-gloss {
		font-size: 1.7rem;
		font-weight: 600;
		text-align: center;
	}
	.fc-hint {
		font-size: 0.8rem;
		color: var(--muted);
	}
	.nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.emoji-big {
		font-size: 3rem;
	}
</style>
