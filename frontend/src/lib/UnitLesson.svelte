<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';
	import { posLabel } from '$lib/pos.js';
	import Exercise from '$lib/Exercise.svelte';

	let { unitId, live = false, lang = 'es', onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let selected = $state(null);
	let revealed = $state(new Set());
	let readRevealed = $state(false);
	let practice = $state(null);
	let practiceLoading = $state(false);
	let practiceRevealed = $state(false);
	let saving = $state(false);
	let result = $state(null); // LessonResult → celebration
	let step = $state(0);

	// The ordered steps of the lesson flow, built from what the unit actually has.
	const STEP_TITLES = {
		objective: 'Goal',
		teach: 'Learn',
		examples: 'Examples',
		read: 'Read',
		practice: 'Practice',
		check: 'Finish'
	};
	let steps = $derived.by(() => {
		if (!lesson) return [];
		const s = ['objective', 'teach', 'examples'];
		if (lesson.reading) s.push('read');
		s.push('practice');
		s.push('check');
		return s;
	});
	let current = $derived(steps[step] ?? null);
	let isLast = $derived(step >= steps.length - 1);

	// Practice quiz over the unit's words (offline, multi-modal).
	let quizItems = $state([]);
	let quizLoading = $state(false);
	let quizLoaded = $state(false);
	let qIdx = $state(0);
	let qAnswered = $state(false);
	let qCorrect = $state(0);
	let qCurrent = $derived(qIdx < quizItems.length ? quizItems[qIdx] : null);
	let quizDone = $derived(quizLoaded && quizItems.length > 0 && qIdx >= quizItems.length);

	// Load the practice quiz the first time the learner reaches that step.
	$effect(() => {
		if (current === 'practice' && !quizLoaded && !quizLoading) loadQuiz();
	});

	async function loadQuiz() {
		quizLoading = true;
		qIdx = 0;
		qAnswered = false;
		qCorrect = 0;
		try {
			quizItems = await api.unitQuiz(unitId);
			quizLoaded = true;
		} catch (e) {
			error = String(e);
		} finally {
			quizLoading = false;
		}
	}
	async function onQuizAnswer(correct) {
		qAnswered = true;
		if (correct) qCorrect += 1;
		try {
			await api.recordExercise(qCurrent.lexeme_id, correct);
		} catch (e) {
			error = String(e);
		}
	}
	function qNext() {
		qAnswered = false;
		qIdx += 1;
	}

	async function load() {
		loading = true;
		error = '';
		try {
			lesson = await api.unitLesson(unitId);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	const plain = (tokens) => tokens.map((t) => t.text).join('');

	function next() {
		selected = null;
		if (!isLast) step += 1;
	}
	function prev() {
		selected = null;
		if (step > 0) step -= 1;
	}

	function pick(t) {
		if (!t.is_word || t.status == null) return;
		selected = selected === t ? null : t;
	}
	async function markKnown(t) {
		if (!t || t.lexeme_id == null) return;
		try {
			await api.setLexemeStatus(t.lexeme_id, 'known');
			t.status = 'known';
			selected = null;
		} catch (e) {
			error = String(e);
		}
	}
	function toggleReveal(i) {
		if (revealed.has(i)) revealed.delete(i);
		else revealed.add(i);
		revealed = new Set(revealed);
	}
	async function studied() {
		saving = true;
		try {
			result = await api.completeUnitLesson(unitId, true);
		} catch (e) {
			error = String(e);
			saving = false;
		}
	}
	async function getPractice() {
		practiceLoading = true;
		practiceRevealed = false;
		try {
			practice = await api.nextContentForUnit(unitId);
		} catch (e) {
			error = String(e);
		} finally {
			practiceLoading = false;
		}
	}
</script>

{#snippet tline(tokens)}
	{#each tokens as t, i (i)}
		{#if t.is_word}<span
				class="tok word {t.status}"
				role="button"
				tabindex="0"
				title={t.gloss ?? ''}
				onclick={() => pick(t)}
				onkeydown={(e) => e.key === 'Enter' && pick(t)}>{t.text}</span
			>{:else}<span class="tok">{t.text}</span>{/if}
	{/each}
{/snippet}

{#snippet meaning()}
	{#if selected}
		<div class="meaning">
			<div>
				<button class="iconbtn" title="Listen" onclick={() => speak(selected.text, lang)}>🔊</button>
				<span class="m-word">{selected.text}</span>
				<span class="m-gloss">{selected.gloss ?? 'no meaning on file'}</span>
			</div>
			{#if selected.lemma}
				<div class="m-lemma">a form of <strong>{selected.lemma}</strong> — its dictionary form</div>
			{/if}
			{#if selected.lexeme_id != null && selected.status !== 'known'}
				<button onclick={() => markKnown(selected)}>✓ I know this word</button>
			{/if}
		</div>
	{/if}
{/snippet}

{#if result}
	<div class="card celebrate">
		<div class="emoji">🎉</div>
		<h2>Lesson complete!</h2>
		<p>
			{result.newly_known} new word{result.newly_known === 1 ? '' : 's'} learned ·
			{result.percent}% of this unit
		</p>
		{#if result.done}<p class="muted">Unit complete — the next one is unlocked.</p>{/if}
		{#if result.streak > 0}<p class="streak-big">🔥 {result.streak}-day streak</p>{/if}
		<button class="primary" onclick={onBack}>Back to roadmap →</button>
	</div>
{:else}
	<button class="link" onclick={onBack}>← Back to roadmap</button>

	{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if lesson}
		<div class="page-head" style="margin-top: 0.8rem;">
			<div class="title-row">
				<h1>{lesson.title}</h1>
				{#if lesson.level}<span class="level-badge">{lesson.level}</span>{/if}
			</div>
			<p>{lesson.description}</p>
		</div>

		<!-- Stepper -->
		<ol class="stepper">
			{#each steps as s, i (s)}
				<li class="stp" class:active={i === step} class:done={i < step}>
					<span class="stp-num">{i < step ? '✓' : i + 1}</span>
					<span class="stp-label">{STEP_TITLES[s]}</span>
				</li>
			{/each}
		</ol>

		<!-- 1. Objective -->
		{#if current === 'objective'}
			<div class="card">
				<div class="step-kicker">Lesson goal</div>
				<h2 class="goal">By the end, you'll be able to…</h2>
				<p class="objective">🎯 {lesson.objective || lesson.description}</p>
				<p class="muted">
					This unit teaches {lesson.words.length} new word{lesson.words.length === 1 ? ' ' : 's '}{#if lesson.grammar}
						and one grammar focus ({lesson.grammar}){/if}. You'll see them, read them in
					context, then check what stuck.
				</p>
			</div>
		{/if}

		<!-- 2. Teach -->
		{#if current === 'teach'}
			{#if lesson.grammar_tip}
				<div class="tip">
					<span class="tip-label">Grammar{#if lesson.grammar} · {lesson.grammar}{/if}</span>
					<p>{lesson.grammar_tip}</p>
				</div>
			{/if}
			<div class="card">
				<div class="step-kicker">New words</div>
				<div class="nw-label">Tap a word to hear it</div>
				<div class="vocab">
					{#each lesson.words as w (w.lexeme_id)}
						<button class="vchip {w.status}" title="Listen" onclick={() => speak(w.lemma, lang)}>
							<strong>{w.lemma}</strong>{#if w.transliteration}<span class="translit">{w.transliteration}</span>{/if}{#if posLabel(w.pos)}<span class="pos-tag">{posLabel(w.pos)}</span>{/if}{#if w.gloss} — {w.gloss}{/if} <span class="spk">🔊</span>
						</button>
					{/each}
				</div>
			</div>

			{#if lesson.conjugations.length}
				<div class="card">
					<div class="step-kicker">How the verbs change</div>
					<p class="muted" style="margin-top: 0;">
						Verbs change their ending for each person. These are the same word — you'll see these
						forms in the examples, so they aren't new vocabulary. A <span class="irr-key">★</span>
						marks an <strong>irregular</strong> form that doesn't follow the regular pattern — learn
						those by heart.
					</p>
					{#each lesson.conjugations as c (c.lemma)}
						<div class="conj">
							<div class="conj-head">
								<strong>{c.lemma}</strong>{#if c.gloss} — {c.gloss}{/if}
								<span class="muted">· present tense</span>
							</div>
							<table class="conj-table">
								<tbody>
									{#each c.cells as cell (cell.pronoun)}
										<tr class:irr={cell.irregular}>
											<td class="conj-pron">{cell.pronoun} <span class="muted">({cell.pronoun_gloss})</span></td>
											<td class="conj-form">
												<button class="link-form" title="Listen" onclick={() => speak(cell.form, lang)}>
													{cell.form}{#if cell.irregular} <span class="irr-key" title="irregular">★</span>{/if} 🔊
												</button>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/each}
				</div>
			{/if}
		{/if}

		<!-- 3. Examples -->
		{#if current === 'examples'}
			<div class="card">
				<div class="step-kicker">See it in use</div>
				<div class="lesson-examples">
					{#each lesson.examples as ex, i (i)}
						<div class="ex">
							<div class="story">{@render tline(ex.tokens)}</div>
							{#if revealed.has(i)}<div class="ex-tr">{ex.translation}</div>{/if}
							<div class="ex-actions">
								<button class="iconbtn" title="Listen" onclick={() => speak(plain(ex.tokens), lang)}>🔊</button>
								<button class="link" onclick={() => toggleReveal(i)}>
									{revealed.has(i) ? 'Hide' : 'Show'} translation
								</button>
							</div>
						</div>
					{/each}
				</div>
				{@render meaning()}
				<div class="legend">
					<span><span class="swatch" style="background: var(--known)"></span>known</span>
					<span><span class="swatch" style="background: var(--partial)"></span>learning</span>
					<span><span class="swatch" style="background: var(--new)"></span>new</span>
					<span class="muted">· tap a word for its meaning, 🔊 to hear it</span>
				</div>
			</div>
		{/if}

		<!-- 4. Read -->
		{#if current === 'read' && lesson.reading}
			<div class="card">
				<div class="step-kicker">Read at your level</div>
				<h2 class="reading-title">{lesson.reading.title}</h2>
				<div class="story reading">{@render tline(lesson.reading.tokens)}</div>
				{#if readRevealed}<div class="ex-tr reading-tr">{lesson.reading.translation}</div>{/if}
				<div class="ex-actions">
					<button class="iconbtn" title="Listen" onclick={() => speak(plain(lesson.reading.tokens), lang)}>🔊</button>
					<button class="link" onclick={() => (readRevealed = !readRevealed)}>
						{readRevealed ? 'Hide' : 'Show'} translation
					</button>
				</div>
				{@render meaning()}
				<p class="muted" style="margin-top: 0.8rem;">
					Tap any word you don't recognise to see its meaning — you don't need to understand every
					word, just the gist.
				</p>
			</div>
		{/if}

		<!-- 5. Practice — quiz on the unit's words -->
		{#if current === 'practice'}
			<div class="card">
				<div class="step-kicker">Practice what you learned</div>
				{#if quizLoading}
					<p class="muted">Building your practice…</p>
				{:else if quizDone}
					<div class="quiz-result">
						<div class="emoji">{qCorrect === quizItems.length ? '🏆' : '✅'}</div>
						<p>You got <strong>{qCorrect}</strong> of <strong>{quizItems.length}</strong> right.</p>
						<button onclick={loadQuiz}>Practice again</button>
					</div>
				{:else if qCurrent}
					<div class="quiz-progress">Question {qIdx + 1} of {quizItems.length}</div>
					{#key qIdx}
						<Exercise item={qCurrent} {lang} onAnswer={onQuizAnswer} />
					{/key}
					{#if qAnswered}
						<div class="row" style="justify-content: flex-end; margin-top: 1.2rem;">
							<button class="primary" onclick={qNext}>
								{qIdx + 1 < quizItems.length ? 'Next →' : 'Finish'}
							</button>
						</div>
					{/if}
				{:else}
					<p class="muted">No words to practice here yet.</p>
				{/if}
			</div>

			{#if live}
				<div class="card" style="margin-top: 1.2rem;">
					<div class="step-kicker">Extra: AI practice sentence</div>
					{#if !practice}
						<p class="muted">A fresh sentence built from this unit's words, at your level.</p>
						<button onclick={getPractice} disabled={practiceLoading}>
							{practiceLoading ? 'Generating…' : 'Generate a sentence'}
						</button>
					{:else}
						<div class="story">{@render tline(practice.tokens)}</div>
						<div class="ex-actions">
							<button class="iconbtn" title="Listen" onclick={() => speak(plain(practice.tokens), lang)}>🔊</button>
							{#if practice.translation}
								<button class="link" onclick={() => (practiceRevealed = !practiceRevealed)}>
									{practiceRevealed ? 'Hide' : 'Reveal'} translation
								</button>
							{/if}
						</div>
						{#if practiceRevealed && practice.translation}<div class="ex-tr">{practice.translation}</div>{/if}
						{@render meaning()}
						<div class="row" style="margin-top: 1rem;">
							<button onclick={getPractice} disabled={practiceLoading}>Another</button>
						</div>
					{/if}
				</div>
			{/if}
		{/if}

		<!-- 6. Check / finish -->
		{#if current === 'check'}
			<div class="card">
				<div class="step-kicker">Check & finish</div>
				<h2 class="goal">Did you reach the goal?</h2>
				<p class="objective">🎯 {lesson.objective || lesson.description}</p>
				<div class="recap">
					{#each lesson.words as w (w.lexeme_id)}
						<button class="vchip {w.status}" title="Listen" onclick={() => speak(w.lemma, lang)}>
							<strong>{w.lemma}</strong>{#if w.gloss} — {w.gloss}{/if}
						</button>
					{/each}
				</div>
				<p class="muted">
					Marking this studied credits these words toward your mastery — they'll come back in Review
					so they stick.
				</p>
			</div>
		{/if}

		<!-- Footer nav -->
		<div class="nav">
			<button class="link" onclick={prev} disabled={step === 0}>← Back</button>
			<span class="nav-count">Step {step + 1} of {steps.length}</span>
			{#if isLast}
				<button class="primary" onclick={studied} disabled={saving}>I've studied this ✓</button>
			{:else}
				<button class="primary" onclick={next}>Continue →</button>
			{/if}
		</div>
	{/if}
{/if}

<style>
	.title-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	.level-badge {
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.03em;
		padding: 0.15rem 0.5rem;
		border-radius: 999px;
		background: var(--panel-2);
		border: 1px solid var(--border);
		color: var(--muted);
	}
	.pos-tag {
		font-size: 0.62rem;
		font-weight: 600;
		color: var(--muted);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 0.03rem 0.32rem;
		margin: 0 0.1rem 0 0.35rem;
		vertical-align: middle;
	}
	.translit {
		color: var(--muted);
		font-style: italic;
		font-size: 0.85rem;
		margin-left: 0.35rem;
	}
	.stepper {
		list-style: none;
		display: flex;
		gap: 0.4rem;
		padding: 0;
		margin: 1rem 0 1.2rem;
		flex-wrap: wrap;
	}
	.stp {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		color: var(--muted);
		opacity: 0.65;
	}
	.stp:not(:last-child)::after {
		content: '';
		width: 1.1rem;
		height: 1px;
		background: var(--border);
		margin-left: 0.4rem;
	}
	.stp.active,
	.stp.done {
		opacity: 1;
	}
	.stp.active .stp-label {
		color: var(--text);
		font-weight: 600;
	}
	.stp-num {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.4rem;
		height: 1.4rem;
		border-radius: 50%;
		background: var(--panel-2);
		border: 1px solid var(--border);
		font-size: 0.72rem;
		font-weight: 700;
	}
	.stp.active .stp-num {
		border-color: var(--accent);
		color: var(--accent);
	}
	.stp.done .stp-num {
		background: var(--accent);
		border-color: var(--accent);
		color: #04201d;
	}
	.step-kicker {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--accent);
		font-weight: 700;
		margin-bottom: 0.5rem;
	}
	.goal {
		margin: 0.2rem 0 0.6rem;
		font-size: 1.15rem;
	}
	.objective {
		font-size: 1.05rem;
		font-weight: 600;
		margin: 0 0 0.8rem;
	}
	.reading-title {
		margin: 0.1rem 0 0.7rem;
		font-size: 1.1rem;
	}
	.story.reading {
		line-height: 2.1;
		font-size: 1.05rem;
	}
	.reading-tr {
		margin-top: 0.7rem;
	}
	.recap {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin: 0.6rem 0 1rem;
	}
	.m-lemma {
		margin-top: 0.35rem;
		font-size: 0.85rem;
		color: var(--muted);
	}
	.conj {
		margin-top: 1rem;
		padding-top: 0.9rem;
		border-top: 1px solid var(--border);
	}
	.conj:first-of-type {
		border-top: none;
		padding-top: 0;
	}
	.conj-head {
		font-size: 0.98rem;
		margin-bottom: 0.4rem;
	}
	.conj-table {
		border-collapse: collapse;
		width: 100%;
		max-width: 22rem;
	}
	.conj-table td {
		padding: 0.22rem 0.4rem;
		border-bottom: 1px solid var(--border);
	}
	.conj-pron {
		color: var(--text);
		width: 55%;
	}
	.conj-form {
		font-weight: 600;
	}
	.link-form {
		background: none;
		border: none;
		color: var(--accent);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
		padding: 0;
	}
	.conj-table tr.irr .conj-form {
		color: var(--new);
	}
	.irr-key {
		color: var(--new);
		font-weight: 700;
	}
	.nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-top: 1.4rem;
	}
	.nav-count {
		font-size: 0.8rem;
		color: var(--muted);
	}
	.quiz-progress {
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		margin-bottom: 0.4rem;
	}
	.quiz-result {
		text-align: center;
		padding: 0.6rem 0;
	}
	.quiz-result .emoji {
		font-size: 2.4rem;
	}
</style>
