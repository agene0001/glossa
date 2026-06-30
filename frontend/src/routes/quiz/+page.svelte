<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import Exercise from '$lib/Exercise.svelte';

	let items = $state([]);
	let lang = $state('es');
	let loading = $state(true);
	let error = $state('');
	let idx = $state(0);
	let answered = $state(false);
	let correctCount = $state(0);
	let streak = $state(0);

	let current = $derived(idx < items.length ? items[idx] : null);
	let finished = $derived(!loading && items.length > 0 && idx >= items.length);

	async function start() {
		loading = true;
		error = '';
		idx = 0;
		answered = false;
		correctCount = 0;
		try {
			const s = await api.backendStatus();
			lang = s.language || 'es';
		} catch {
			/* ignore */
		}
		try {
			items = await api.reviewSession(12);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(start);

	async function onAnswer(correct) {
		answered = true;
		if (correct) correctCount += 1;
		try {
			const r = await api.recordExercise(current.lexeme_id, correct);
			streak = r.streak;
		} catch (e) {
			error = String(e);
		}
	}
	function next() {
		answered = false;
		idx += 1;
	}
</script>

<div class="page-head">
	<h1>Quiz</h1>
	<p>Spaced-repetition review — your weakest words come first, mixing recognition and recall.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if items.length === 0}
	<div class="card">
		<p>Nothing to review yet — study a unit on the <a href="/">Learn</a> tab or a pack on
			<a href="/vocab">Vocab</a> first, then come back to test yourself.</p>
	</div>
{:else if finished}
	<div class="card celebrate">
		<div class="emoji">{correctCount === items.length ? '🏆' : '✅'}</div>
		<h2>Review complete!</h2>
		<p>You got <strong>{correctCount}</strong> of <strong>{items.length}</strong> right.</p>
		{#if streak > 0}<p class="streak-big">🔥 {streak}-day streak</p>{/if}
		<button class="primary" onclick={start}>Review again</button>
	</div>
{:else if current}
	<div class="card quiz">
		<div class="quiz-progress">Question {idx + 1} of {items.length}</div>
		{#key idx}
			<Exercise item={current} {lang} {onAnswer} />
		{/key}
		{#if answered}
			<div class="row" style="justify-content: flex-end; margin-top: 1.2rem;">
				<button class="primary" onclick={next}>
					{idx + 1 < items.length ? 'Next →' : 'Finish'}
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.quiz-progress {
		font-size: 0.78rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		margin-bottom: 0.4rem;
	}
</style>
