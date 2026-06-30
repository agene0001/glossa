<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';

	let guide = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			guide = await api.pronunciationGuide();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	// Group entries by category, preserving order.
	let grouped = $derived.by(() => {
		if (!guide) return [];
		const out = [];
		for (const e of guide.entries) {
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
	<p>How the language is pronounced — the letters and sounds that differ from English. Tap a word
		to hear it.</p>
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

	{#each grouped as g (g.category)}
		<div class="cat">{g.category}</div>
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
