<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';

	let items = $state([]);
	let lang = $state('es');
	let loading = $state(true);
	let error = $state('');
	let idx = $state(0);
	let chosen = $state(null); // chosen option index, null until answered
	let correctCount = $state(0);
	let streak = $state(0);

	let current = $derived(idx < items.length ? items[idx] : null);
	let finished = $derived(!loading && items.length > 0 && idx >= items.length);

	async function start() {
		loading = true;
		error = '';
		idx = 0;
		chosen = null;
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

	async function choose(i) {
		if (chosen !== null || !current) return;
		chosen = i;
		const correct = i === current.answer_index;
		if (correct) correctCount += 1;
		speak(current.prompt, lang);
		try {
			const r = await api.recordExercise(current.lexeme_id, correct);
			streak = r.streak;
		} catch (e) {
			error = String(e);
		}
	}

	function next() {
		chosen = null;
		idx += 1;
	}

	function optionClass(i) {
		if (chosen === null) return '';
		if (i === current.answer_index) return 'correct';
		if (i === chosen) return 'wrong';
		return 'dim';
	}
</script>

<div class="page-head">
	<h1>Quiz</h1>
	<p>Spaced-repetition review — your weakest words come first. Pick the meaning of each word.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if items.length === 0}
	<div class="card">
		<p>Nothing to review yet — study a unit on the <a href="/">Learn</a> tab first, then come back to
			test yourself.</p>
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

		<div class="prompt-row">
			<div class="prompt">{current.prompt}</div>
			<button class="iconbtn" title="Listen" onclick={() => speak(current.prompt, lang)}>🔊</button>
		</div>
		<div class="prompt-sub">What does this mean?</div>

		<div class="options">
			{#each current.options as opt, i (i)}
				<button class="option {optionClass(i)}" disabled={chosen !== null} onclick={() => choose(i)}>
					{opt}
				</button>
			{/each}
		</div>

		{#if chosen !== null}
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
	}
	.prompt-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-top: 0.8rem;
	}
	.prompt {
		font-size: 2.2rem;
		font-weight: 700;
	}
	.prompt-sub {
		color: var(--muted);
		margin-top: 0.2rem;
	}
	.options {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.7rem;
		margin-top: 1.4rem;
	}
	@media (max-width: 560px) {
		.options {
			grid-template-columns: 1fr;
		}
	}
	.option {
		text-align: left;
		padding: 0.9rem 1rem;
		font-size: 1rem;
		border-radius: 11px;
	}
	.option.correct {
		background: var(--known);
		border-color: var(--known);
		color: #04110c;
	}
	.option.wrong {
		background: rgba(224, 106, 106, 0.18);
		border-color: var(--unknown);
	}
	.option.dim {
		opacity: 0.55;
	}
</style>
