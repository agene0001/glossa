<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let content = $state(null);
	let loading = $state(false);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			content = await api.nextContent();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function finish(understood) {
		if (!content) return;
		try {
			await api.recordStoryRead(content.story_id, understood);
		} catch (e) {
			error = String(e);
			return;
		}
		await load();
	}

	async function markKnown(tok) {
		if (!tok.is_word || tok.lexeme_id == null || tok.status === 'known') return;
		try {
			await api.setLexemeStatus(tok.lexeme_id, 'known');
			tok.status = 'known'; // $state proxy → updates in place
		} catch (e) {
			error = String(e);
		}
	}

	onMount(load);
</script>

<div class="page-head">
	<h1>Reading</h1>
	<p>Comprehensible input built from what you already know. Tap any word to mark it known.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

<div class="card">
	{#if loading && !content}
		<p class="muted">Generating…</p>
	{:else if content}
		<div class="story">
			{#each content.tokens as tok, i (i)}
				{#if tok.is_word}
					<span
						class="tok word {tok.status}"
						role="button"
						tabindex="0"
						title={tok.lexeme_id != null ? 'Click to mark as known' : ''}
						onclick={() => markKnown(tok)}
						onkeydown={(e) => e.key === 'Enter' && markKnown(tok)}>{tok.text}</span>
				{:else}<span class="tok">{tok.text}</span>{/if}
			{/each}
		</div>

		{#if content.new_words.length}
			<div class="glossary">
				{#each content.new_words as w (w.lemma)}
					<span class="chip">{w.lemma}{#if w.pos} · {w.pos}{/if}</span>
				{/each}
			</div>
		{/if}

		<div class="legend">
			<span><span class="swatch" style="background: var(--known)"></span>known</span>
			<span><span class="swatch" style="background: var(--partial)"></span>learning</span>
			<span><span class="swatch" style="background: var(--new)"></span>new</span>
			<span><span class="swatch" style="background: var(--unknown)"></span>unknown</span>
			<span class="muted">· {Math.round(content.known_ratio * 100)}% known</span>
			{#if content.grammar_targeted}
				<span class="muted">· focus: {content.grammar_targeted}</span>
			{/if}
		</div>

		<div class="row" style="margin-top: 1.6rem;">
			<button class="primary" onclick={() => finish(true)} disabled={loading}>
				I understood it ✓
			</button>
			<button onclick={() => finish(false)} disabled={loading}>Too hard — skip</button>
		</div>
	{:else}
		<p class="muted">No content yet.</p>
	{/if}
</div>
