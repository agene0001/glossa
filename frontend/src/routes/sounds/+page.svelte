<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';

	let guide = $state(null);
	let loading = $state(true);
	let error = $state('');

	// Top-level views: the alphabet, the numbers, and the condensed
	// special-cases reference (everything else).
	const VIEWS = [
		{ key: 'alphabet', label: 'Alphabet', match: (c) => c === 'Alphabet' },
		{ key: 'numbers', label: 'Numbers', match: (c) => c === 'Numbers' },
		{ key: 'sounds', label: 'Sounds', match: (c) => c !== 'Alphabet' && c !== 'Numbers' }
	];
	let view = $state('alphabet');

	async function load() {
		loading = true;
		error = '';
		try {
			guide = await api.pronunciationGuide();
			// Default to the first view this language actually has.
			const first = VIEWS.find((v) => guide?.entries.some((e) => v.match(e.category)));
			if (first) view = first.key;
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	let availableViews = $derived(
		guide ? VIEWS.filter((v) => guide.entries.some((e) => v.match(e.category))) : []
	);

	// Entries of the active view, grouped by their (sub-)category in order.
	let grouped = $derived.by(() => {
		if (!guide) return [];
		const v = VIEWS.find((x) => x.key === view);
		if (!v) return [];
		const out = [];
		for (const e of guide.entries.filter((e) => v.match(e.category))) {
			let g = out.find((x) => x.category === e.category);
			if (!g) {
				g = { category: e.category, items: [] };
				out.push(g);
			}
			g.items.push(e);
		}
		return out;
	});
	const lang = $derived(guide?.language ?? 'es');
</script>

<div class="page-head">
	<h1>Sounds &amp; spelling</h1>
	<p>The alphabet, the numbers, and the sounds that differ from English — switch between them below.
		Tap any letter, number, or word to hear it.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if !guide}
	<div class="card">
		<p class="muted">No pronunciation guide for this language yet.</p>
	</div>
{:else}
	{#if guide.intro}
		<div class="card intro">{guide.intro}</div>
	{/if}

	{#if availableViews.length > 1}
		<div class="toggle">
			{#each availableViews as v (v.key)}
				<button class:active={view === v.key} onclick={() => (view = v.key)}>{v.label}</button>
			{/each}
		</div>
	{/if}

	{#each grouped as g (g.category)}
		{#if grouped.length > 1}<div class="cat">{g.category}</div>{/if}
		<div class="card sounds">
			{#each g.items as e (e.symbol)}
				<div class="row">
					{#if e.say}
						<button class="symbol speak" title="Play" onclick={() => speak(e.say, lang)}>{e.symbol}</button>
					{:else}
						<div class="symbol">{e.symbol}</div>
					{/if}
					<div class="how">{e.sound}</div>
					{#if e.example}
						<button class="ex" title="Listen" onclick={() => speak(e.example, lang)}>
							<span class="ex-word">{e.example}</span>
							{#if e.example_gloss}<span class="ex-gloss">{e.example_gloss}</span>{/if}
							<span class="spk">🔊</span>
						</button>
					{:else if e.example_gloss}
						<span class="ex-gloss only">{e.example_gloss}</span>
					{/if}
				</div>
			{/each}
		</div>
	{/each}
{/if}

<style>
	.intro {
		line-height: 1.55;
	}
	.toggle {
		display: inline-flex;
		gap: 0.25rem;
		padding: 0.25rem;
		background: var(--panel-2);
		border: 1px solid var(--border);
		border-radius: 11px;
		margin: 1.2rem 0 0.4rem;
	}
	.toggle button {
		background: none;
		border: none;
		color: var(--muted);
		font: inherit;
		font-weight: 600;
		padding: 0.4rem 1rem;
		border-radius: 8px;
		cursor: pointer;
	}
	.toggle button.active {
		background: var(--accent);
		color: #04201d;
	}
	.cat {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
		font-weight: 700;
		margin: 1.4rem 0 0.5rem;
	}
	.sounds {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.row {
		display: grid;
		grid-template-columns: 4.5rem 1fr auto;
		align-items: center;
		gap: 1rem;
		padding: 0.55rem 0;
		border-bottom: 1px solid var(--border);
	}
	.row:last-child {
		border-bottom: none;
	}
	.symbol {
		font-size: 1.5rem;
		font-weight: 700;
	}
	.symbol.speak {
		background: var(--panel-2);
		border: 1px solid var(--border);
		border-radius: 10px;
		color: var(--text);
		cursor: pointer;
		padding: 0.25rem 0;
		font-family: inherit;
	}
	.symbol.speak:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.ex-gloss.only {
		color: var(--muted);
		font-size: 0.9rem;
	}
	.how {
		color: var(--text);
		font-size: 0.95rem;
	}
	.ex {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		background: var(--panel-2);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 0.4rem 0.7rem;
		cursor: pointer;
		text-align: left;
	}
	.ex:hover {
		border-color: var(--accent);
	}
	.ex-word {
		font-weight: 600;
	}
	.ex-gloss {
		color: var(--muted);
		font-size: 0.82rem;
	}
	.spk {
		font-size: 0.85rem;
	}
	@media (max-width: 560px) {
		.row {
			grid-template-columns: 3.5rem 1fr;
		}
		.ex {
			grid-column: 1 / -1;
		}
	}
</style>
