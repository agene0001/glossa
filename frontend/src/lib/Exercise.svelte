<script>
	// One exercise, any kind. Multiple-choice (choose_meaning / choose_word) or
	// typed production (type_answer). Owns its answered state and feedback; calls
	// onAnswer(correct) once so the parent can record + advance.
	import { speak } from '$lib/audio.js';
	import { posLabel } from '$lib/pos.js';

	let { item, lang = 'es', onAnswer } = $props();

	let answered = $state(false);
	let chosen = $state(null);
	let typed = $state('');
	let correct = $state(false);

	// Match the backend's lenient checking: lowercase, trim, strip diacritics.
	const norm = (s) => s.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
	const isMC = $derived(item.kind === 'choose_meaning' || item.kind === 'choose_word');
	// The target word (to pronounce): the prompt for recognition, else the answer.
	const word = $derived(item.kind === 'choose_meaning' ? item.prompt : item.answer);

	function chooseMC(i) {
		if (answered) return;
		chosen = i;
		correct = i === item.answer_index;
		finish();
	}
	function submitTyped() {
		if (answered || !typed.trim()) return;
		const n = norm(typed);
		correct = item.accepts.includes(n) || n === norm(item.answer);
		finish();
	}
	function finish() {
		answered = true;
		speak(word, lang);
		onAnswer(correct);
	}
	function optionClass(i) {
		if (!answered) return '';
		if (i === item.answer_index) return 'correct';
		if (i === chosen) return 'wrong';
		return 'dim';
	}
	function onKey(e) {
		if (e.key === 'Enter') submitTyped();
	}
</script>

<div class="ex">
	<div class="instruction">{item.instruction}</div>
	<div class="prompt-row">
		<div class="prompt">{item.prompt}</div>
		{#if posLabel(item.pos)}<span class="pos-tag">{posLabel(item.pos)}</span>{/if}
		{#if item.kind === 'choose_meaning'}
			<button class="iconbtn" title="Listen" onclick={() => speak(word, lang)}>🔊</button>
		{/if}
	</div>

	{#if isMC}
		<div class="options">
			{#each item.options as opt, i (i)}
				<button class="option {optionClass(i)}" disabled={answered} onclick={() => chooseMC(i)}>
					{opt}
				</button>
			{/each}
		</div>
	{:else}
		<div class="type-row">
			<!-- svelte-ignore a11y_autofocus -->
			<input
				placeholder="type in {lang}…"
				bind:value={typed}
				disabled={answered}
				onkeydown={onKey}
				autofocus />
			{#if !answered}
				<button class="primary" onclick={submitTyped} disabled={!typed.trim()}>Check</button>
			{/if}
		</div>
	{/if}

	{#if answered}
		<div class="feedback {correct ? 'ok' : 'no'}">
			{#if correct}
				✓ Correct
			{:else}
				✗ Answer: <strong>{item.answer}</strong>
			{/if}
			{#if item.kind !== 'choose_meaning'}
				<button class="iconbtn" title="Listen" onclick={() => speak(word, lang)}>🔊</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	.instruction {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--accent);
		font-weight: 700;
	}
	.prompt-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-top: 0.6rem;
	}
	.prompt {
		font-size: 2rem;
		font-weight: 700;
	}
	.pos-tag {
		font-size: 0.68rem;
		font-weight: 600;
		color: var(--muted);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 0.1rem 0.45rem;
	}
	.options {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.7rem;
		margin-top: 1.3rem;
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
	.type-row {
		display: flex;
		gap: 0.6rem;
		margin-top: 1.3rem;
	}
	.type-row input {
		flex: 1;
		padding: 0.8rem 0.9rem;
		border-radius: 11px;
		border: 1px solid var(--border);
		background: var(--panel-2);
		color: var(--text);
		font: inherit;
		font-size: 1.1rem;
	}
	.feedback {
		margin-top: 1rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
	}
	.feedback.ok {
		color: var(--known);
	}
	.feedback.no {
		color: var(--unknown);
	}
</style>
