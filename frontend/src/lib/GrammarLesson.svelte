<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import Exercise from '$lib/Exercise.svelte';

	let { patternId, lang = 'es', onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let phase = $state('learn'); // 'learn' | 'drill' | 'done'

	let dIdx = $state(0);
	let answered = $state(false);
	let correctCount = $state(0);
	let streak = $state(0);

	let drill = $derived(lesson && dIdx < lesson.drills.length ? lesson.drills[dIdx] : null);
	// A grammar drill is a typed cloze — render it through the shared Exercise.
	let item = $derived(
		drill && {
			kind: 'type_answer',
			instruction: 'Fill in the blank',
			prompt: drill.prompt,
			options: [],
			answer_index: 0,
			answer: drill.answer,
			accepts: drill.accepts
		}
	);

	async function load() {
		loading = true;
		error = '';
		try {
			lesson = await api.grammarLesson(patternId);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	function startDrills() {
		if (lesson.drills.length) {
			phase = 'drill';
			dIdx = 0;
			answered = false;
			correctCount = 0;
		}
	}
	async function onAnswer(correct) {
		answered = true;
		if (correct) correctCount += 1;
		try {
			const r = await api.recordGrammarExercise(lesson.id, correct);
			streak = r.streak;
		} catch (e) {
			error = String(e);
		}
	}
	function next() {
		answered = false;
		if (dIdx + 1 < lesson.drills.length) dIdx += 1;
		else phase = 'done';
	}
	function restart() {
		phase = 'learn';
		load();
	}
</script>

<button class="link" onclick={onBack}>← All grammar</button>

{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if lesson}
	<div class="page-head" style="margin-top: 0.8rem;">
		<h1>{lesson.title}</h1>
	</div>

	{#if phase === 'learn'}
		<div class="card">
			<div class="step-kicker">The rule</div>
			{#if lesson.explanation}<p class="expl">{lesson.explanation}</p>{/if}
			{#if lesson.example}<div class="example">{lesson.example}</div>{/if}
		</div>
		<div class="nav">
			{#if lesson.drills.length}
				<button class="primary" onclick={startDrills}>Practice drills →</button>
			{:else}
				<button class="primary" onclick={onBack}>Back to grammar →</button>
			{/if}
		</div>
	{/if}

	{#if phase === 'drill' && drill}
		<div class="card">
			<div class="step-kicker">Drill {dIdx + 1} of {lesson.drills.length}</div>
			{#key dIdx}
				<Exercise {item} {lang} {onAnswer} />
			{/key}
			{#if answered}
				<div class="translation">{drill.translation}</div>
				<div class="row" style="justify-content: flex-end; margin-top: 1rem;">
					<button class="primary" onclick={next}>
						{dIdx + 1 < lesson.drills.length ? 'Next →' : 'Finish'}
					</button>
				</div>
			{/if}
		</div>
	{/if}

	{#if phase === 'done'}
		<div class="card celebrate">
			<div class="emoji-big">{correctCount === lesson.drills.length ? '🏆' : '✅'}</div>
			<h2>Drills complete!</h2>
			<p>You got <strong>{correctCount}</strong> of <strong>{lesson.drills.length}</strong> right.</p>
			{#if streak > 0}<p class="streak-big">🔥 {streak}-day streak</p>{/if}
			<div class="row" style="justify-content: center; gap: 0.7rem;">
				<button onclick={restart}>Again</button>
				<button class="primary" onclick={onBack}>Back to grammar →</button>
			</div>
		</div>
	{/if}
{/if}

<style>
	.step-kicker {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--accent);
		font-weight: 700;
		margin-bottom: 0.5rem;
	}
	.expl {
		font-size: 1.05rem;
		margin: 0 0 0.8rem;
	}
	.example {
		font-style: italic;
		color: var(--muted);
		padding: 0.6rem 0.9rem;
		border-left: 3px solid var(--border);
	}
	.translation {
		margin-top: 0.9rem;
		color: var(--muted);
		font-style: italic;
	}
	.nav {
		margin-top: 1.2rem;
	}
	.emoji-big {
		font-size: 3rem;
	}
</style>
