<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';
	import { posLabel, POS_OPTIONS } from '$lib/pos.js';
	import StudyQuiz from '$lib/StudyQuiz.svelte';

	let { deckId, lang = 'es', live = false, onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let mode = $state('edit'); // 'edit' | 'play'
	let busy = $state(false);

	let newLemma = $state('');
	let newGloss = $state('');
	let newPos = $state('');

	// "Add from English" state.
	let aiQuery = $state('');
	let aiBusy = $state(false);
	let aiAdding = $state(false);
	let aiError = $state('');
	let suggestions = $state([]);

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
			await api.addDeckWord(deckId, lemma, newGloss.trim(), newPos || null);
			newLemma = '';
			newGloss = '';
			newPos = '';
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
	async function suggest(count) {
		const q = aiQuery.trim();
		if (!q || aiBusy) return;
		aiBusy = true;
		aiError = '';
		suggestions = [];
		try {
			suggestions = await api.suggestWords(q, count);
		} catch (e) {
			aiError = String(e);
		} finally {
			aiBusy = false;
		}
	}
	async function addSuggestion(s) {
		aiAdding = true;
		try {
			await api.addDeckWord(deckId, s.term, s.gloss, s.pos);
			suggestions = suggestions.filter((x) => x !== s);
			await load();
		} catch (e) {
			aiError = String(e);
		} finally {
			aiAdding = false;
		}
	}
	async function addAll() {
		aiAdding = true;
		try {
			for (const s of suggestions) await api.addDeckWord(deckId, s.term, s.gloss, s.pos);
			suggestions = [];
			await load();
		} catch (e) {
			aiError = String(e);
		} finally {
			aiAdding = false;
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
		<div class="card ai">
			<div class="nw-label">✨ Add from English</div>
			<div class="add-row">
				<input
					placeholder={live ? 'an English word, or a topic like “kitchen”' : 'an English word, e.g. “umbrella”'}
					bind:value={aiQuery}
					onkeydown={(e) => e.key === 'Enter' && suggest(0)}
					disabled={aiBusy} />
				<button class="primary" onclick={() => suggest(0)} disabled={aiBusy || !aiQuery.trim()}>Translate</button>
				{#if live}
					<button onclick={() => suggest(8)} disabled={aiBusy || !aiQuery.trim()}>Suggest a set</button>
				{/if}
			</div>
			<p class="muted hint">
				{#if live}
					Type a word to translate it, or a topic to get a set of related words.
				{:else}
					Translated from a built-in dictionary — no setup needed. Set <code>ANTHROPIC_API_KEY</code>
					for topics and rarer words.
				{/if}
			</p>
			{#if aiError}<div class="error" style="margin-top: 0.5rem;">{aiError}</div>{/if}
			{#if aiBusy}<p class="muted" style="margin-top: 0.6rem;">Looking it up…</p>{/if}
			{#if suggestions.length}
				<ul class="suggestions">
					{#each suggestions as s (s.term)}
						<li class="sug">
							<button class="iconbtn" title="Listen" onclick={() => speak(s.term, lang)}>🔊</button>
							<span class="sug-term">{s.term}</span>
							<span class="sug-gloss">{s.gloss}{#if s.pos} · {s.pos}{/if}</span>
							<button class="add-btn" onclick={() => addSuggestion(s)} disabled={aiAdding}>+ Add</button>
						</li>
					{/each}
				</ul>
				<button class="link" onclick={addAll} disabled={aiAdding}>Add all {suggestions.length} →</button>
			{/if}
		</div>

		<div class="card">
			<div class="nw-label">Add a word manually</div>
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
				<select class="pos-select" bind:value={newPos} disabled={busy} aria-label="word type">
					{#each POS_OPTIONS as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
				</select>
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
							{#if posLabel(w.pos)}<span class="pos-tag">{posLabel(w.pos)}</span>{/if}
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
	.ai {
		border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
	}
	.hint {
		font-size: 0.82rem;
		margin-top: 0.5rem;
	}
	.suggestions {
		list-style: none;
		margin: 0.9rem 0 0.6rem;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.sug {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.4rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: 9px;
		background: var(--panel-2);
	}
	.sug-term {
		font-weight: 600;
	}
	.sug-gloss {
		color: var(--muted);
		flex: 1;
		font-size: 0.9rem;
	}
	.add-btn {
		font-size: 0.85rem;
		padding: 0.3rem 0.7rem;
	}
	.pos-select {
		padding: 0.6rem 0.5rem;
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
	.pos-tag {
		font-size: 0.68rem;
		font-weight: 600;
		text-transform: lowercase;
		color: var(--muted);
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 0.05rem 0.4rem;
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
