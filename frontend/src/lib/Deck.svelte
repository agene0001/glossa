<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';
	import StudyQuiz from '$lib/StudyQuiz.svelte';

	let { deckId, lang = 'es', onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let mode = $state('edit'); // 'edit' | 'play'
	let busy = $state(false);

	let newLemma = $state('');
	let newGloss = $state('');

	async function load() {
		error = '';
		try {
			lesson = await api.deckLesson(deckId);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	async function addWord() {
		const lemma = newLemma.trim();
		if (!lemma || busy) return;
		busy = true;
		try {
			await api.addDeckWord(deckId, lemma, newGloss.trim());
			newLemma = '';
			newGloss = '';
			await load();
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}
	function onKey(e) {
		if (e.key === 'Enter') addWord();
	}
	async function removeWord(lexemeId) {
		busy = true;
		try {
			await api.removeDeckWord(deckId, lexemeId);
			await load();
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}
	function play() {
		if (lesson && lesson.cards.length) mode = 'play';
	}
	function backToEdit() {
		mode = 'edit';
		load();
	}
</script>

<button class="link" onclick={onBack}>← All decks</button>

{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if lesson}
	<div class="page-head" style="margin-top: 0.8rem;">
		<h1><span class="emoji">{lesson.emoji}</span> {lesson.title}</h1>
		<p>{lesson.description} · your own words</p>
	</div>

	{#if mode === 'play'}
		<StudyQuiz {lesson} {lang} loadQuiz={() => api.deckQuiz(deckId, 12)} onExit={backToEdit} exitLabel="Back to deck →" />
	{:else}
		<div class="card">
			<div class="nw-label">Add a word</div>
			<div class="add-row">
				<input
					placeholder="word ({lang})"
					bind:value={newLemma}
					onkeydown={onKey}
					disabled={busy} />
				<input
					placeholder="meaning"
					bind:value={newGloss}
					onkeydown={onKey}
					disabled={busy} />
				<button class="primary" onclick={addWord} disabled={busy || !newLemma.trim()}>Add</button>
			</div>
			<p class="muted" style="margin-top: 0.5rem; font-size: 0.82rem;">
				Type a word and its meaning — these become flashcards and feed your review like any other word.
			</p>
		</div>

		<div class="card" style="margin-top: 1.2rem;">
			<div class="row" style="justify-content: space-between; align-items: center;">
				<div class="nw-label" style="margin: 0;">{lesson.cards.length} word{lesson.cards.length === 1 ? '' : 's'}</div>
				<button class="primary" onclick={play} disabled={lesson.cards.length === 0}>Study &amp; quiz →</button>
			</div>

			{#if lesson.cards.length === 0}
				<p class="muted" style="margin-top: 0.8rem;">No words yet — add your first above.</p>
			{:else}
				<ul class="words">
					{#each lesson.cards as w (w.lexeme_id)}
						<li class="word {w.status}">
							<button class="iconbtn" title="Listen" onclick={() => speak(w.lemma, lang)}>🔊</button>
							<span class="w-lemma">{w.lemma}</span>
							<span class="w-gloss">{w.gloss ?? '—'}</span>
							<button class="del" title="Remove" onclick={() => removeWord(w.lexeme_id)} disabled={busy}>✕</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
{/if}

<style>
	.emoji {
		font-size: 1.4rem;
	}
	.add-row {
		display: flex;
		gap: 0.5rem;
	}
	.add-row input {
		flex: 1;
		padding: 0.6rem 0.7rem;
		border-radius: 9px;
		border: 1px solid var(--border);
		background: var(--panel-2);
		color: var(--text);
		font: inherit;
	}
	.words {
		list-style: none;
		margin: 0.8rem 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.word {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: 9px;
		background: var(--panel-2);
	}
	.word.known {
		border-color: var(--known);
	}
	.word.partial {
		border-color: var(--partial);
	}
	.w-lemma {
		font-weight: 600;
	}
	.w-gloss {
		color: var(--muted);
		flex: 1;
	}
	.del {
		background: none;
		border: none;
		color: var(--muted);
		cursor: pointer;
		font-size: 0.9rem;
		padding: 0.2rem 0.4rem;
	}
	.del:hover {
		color: var(--unknown);
	}
</style>
