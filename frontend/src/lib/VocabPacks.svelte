<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let { onOpen } = $props();

	let packs = $state([]);
	let error = $state('');
	let loading = $state(true);

	async function load() {
		loading = true;
		error = '';
		try {
			packs = await api.vocabPacks();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	const cta = (p) => (p.percent >= 80 ? 'Review' : p.percent > 0 ? 'Keep going' : 'Start');
</script>

<div class="page-head">
	<h1>Vocabulary</h1>
	<p>Themed word packs to grow your vocabulary — pick whatever interests you. These feed the same
		progress and review as your lessons.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else}
	<div class="grid">
		{#each packs as p (p.id)}
			<button class="pack" onclick={() => onOpen(p.id)}>
				<div class="pack-top">
					<span class="emoji">{p.emoji}</span>
					<span class="count">{p.known}/{p.total}</span>
				</div>
				<div class="pack-title">{p.title}</div>
				<div class="pack-desc">{p.description}</div>
				<div class="bar">
					<span class="known" style="flex-basis: {p.percent}%"></span>
					<span class="unknown" style="flex-basis: {100 - p.percent}%"></span>
				</div>
				<div class="pack-cta">{cta(p)} →</div>
			</button>
		{/each}
	</div>
{/if}

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 1rem;
	}
	.pack {
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1.1rem 1.2rem;
		cursor: pointer;
		transition: border-color 0.15s, transform 0.05s;
	}
	.pack:hover {
		border-color: var(--accent);
	}
	.pack:active {
		transform: translateY(1px);
	}
	.pack-top {
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
	.pack-title {
		font-size: 1.1rem;
		font-weight: 600;
	}
	.pack-desc {
		color: var(--muted);
		font-size: 0.85rem;
		flex: 1;
	}
	.bar {
		margin-top: 0.5rem;
	}
	.pack-cta {
		margin-top: 0.4rem;
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--accent);
	}
</style>
