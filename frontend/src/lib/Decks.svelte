<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let { onOpen } = $props();

	let decks = $state([]);
	let error = $state('');
	let loading = $state(true);
	let creating = $state(false);
	let newTitle = $state('');
	let newEmoji = $state('📒');
	let busy = $state(false);

	async function load() {
		error = '';
		try {
			decks = await api.listDecks();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	async function create() {
		const title = newTitle.trim();
		if (!title || busy) return;
		busy = true;
		try {
			const deck = await api.createDeck(title, newEmoji.trim() || '📒');
			newTitle = '';
			newEmoji = '📒';
			creating = false;
			onOpen(deck.id); // jump straight into the new deck to add words
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="head">
	<h2>Your decks</h2>
	{#if !creating}
		<button class="primary" onclick={() => (creating = true)}>＋ New deck</button>
	{/if}
</div>
<p class="muted intro">Make your own flashcard sets — the vocab from a class or textbook. They study and
	review just like the built-in packs.</p>

{#if error}<div class="error">{error}</div>{/if}

{#if creating}
	<div class="card create">
		<div class="add-row">
			<input class="emoji-in" maxlength="2" bind:value={newEmoji} aria-label="emoji" />
			<input
				placeholder="Deck name (e.g. German 101 — Chapter 3)"
				bind:value={newTitle}
				onkeydown={(e) => e.key === 'Enter' && create()} />
			<button class="primary" onclick={create} disabled={busy || !newTitle.trim()}>Create</button>
			<button class="link" onclick={() => (creating = false)}>Cancel</button>
		</div>
	</div>
{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if decks.length === 0 && !creating}
	<p class="muted">No decks yet. Create one to add your own words.</p>
{:else}
	<div class="grid">
		{#each decks as d (d.id)}
			<button class="deck" onclick={() => onOpen(d.id)}>
				<div class="deck-top">
					<span class="emoji">{d.emoji}</span>
					<span class="count">{d.known}/{d.total}</span>
				</div>
				<div class="deck-title">{d.title}</div>
				<div class="bar">
					<span class="known" style="flex-basis: {d.percent}%"></span>
					<span class="unknown" style="flex-basis: {100 - d.percent}%"></span>
				</div>
				<div class="deck-cta">{d.total === 0 ? 'Add words' : 'Open'} →</div>
			</button>
		{/each}
	</div>
{/if}

<style>
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 2rem;
	}
	.head h2 {
		margin: 0;
		font-size: 1.3rem;
	}
	.intro {
		margin-top: 0.3rem;
	}
	.create {
		margin-bottom: 1rem;
	}
	.add-row {
		display: flex;
		gap: 0.5rem;
		align-items: center;
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
	.emoji-in {
		flex: 0 0 3rem !important;
		text-align: center;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 1rem;
	}
	.deck {
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1.1rem 1.2rem;
		cursor: pointer;
	}
	.deck:hover {
		border-color: var(--accent);
	}
	.deck-top {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.emoji {
		font-size: 1.8rem;
	}
	.count {
		font-size: 0.78rem;
		color: var(--muted);
	}
	.deck-title {
		font-size: 1.05rem;
		font-weight: 600;
	}
	.deck-cta {
		margin-top: 0.3rem;
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--accent);
	}
</style>
