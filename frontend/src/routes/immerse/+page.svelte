<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let guide = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			guide = await api.externalResources();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	// Fixed display order for the categories.
	const ORDER = ['Watch', 'Listen', 'Read', 'Speak & exchange', 'Tools'];
	let grouped = $derived.by(() => {
		if (!guide) return [];
		const out = [];
		for (const cat of ORDER) {
			const items = guide.resources.filter((r) => r.category === cat);
			if (items.length) out.push({ cat, items });
		}
		// any categories not in ORDER, appended
		for (const r of guide.resources) {
			if (!ORDER.includes(r.category) && !out.some((g) => g.cat === r.category)) {
				out.push({ cat: r.category, items: guide.resources.filter((x) => x.category === r.category) });
			}
		}
		return out;
	});

	async function open(url) {
		try {
			await api.openExternal(url);
		} catch (e) {
			error = String(e);
		}
	}
</script>

<div class="page-head">
	<h1>Immerse</h1>
	<p>Glossa builds your vocabulary and grammar — but real fluency needs <em>volume</em>: hours of
		listening, reading, and talking with people. These are the best <strong>free</strong> ways to
		get it. Curated, opens in your browser.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if !guide}
	<div class="card"><p class="muted">No resources for this language yet.</p></div>
{:else}
	{#each grouped as g (g.cat)}
		<div class="cat">{g.cat}</div>
		<div class="grid">
			{#each g.items as r (r.url)}
				<button class="res" onclick={() => open(r.url)}>
					<div class="res-top">
						<span class="res-title">{r.title}</span>
						<span class="tag">{r.tag}</span>
					</div>
					<div class="res-desc">{r.description}</div>
					<div class="res-link">Open ↗</div>
				</button>
			{/each}
		</div>
	{/each}
{/if}

<style>
	.cat {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
		font-weight: 700;
		margin: 1.4rem 0 0.6rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 0.8rem;
	}
	.res {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		text-align: left;
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 13px;
		padding: 1rem 1.1rem;
		cursor: pointer;
		font: inherit;
		color: var(--text);
	}
	.res:hover {
		border-color: var(--accent);
	}
	.res-top {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.res-title {
		font-weight: 600;
		font-size: 1.02rem;
	}
	.tag {
		font-size: 0.64rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--muted);
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 0.08rem 0.45rem;
		white-space: nowrap;
	}
	.res-desc {
		color: var(--muted);
		font-size: 0.9rem;
		line-height: 1.45;
	}
	.res-link {
		margin-top: auto;
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--accent);
	}
</style>
