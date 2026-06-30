<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let { onOpen } = $props();

	let items = $state([]);
	let error = $state('');
	let loading = $state(true);

	async function load() {
		loading = true;
		error = '';
		try {
			items = await api.grammarTrack();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	const dot = (s) => (s === 'done' ? '✓' : s === 'locked' ? '🔒' : '▸');
	const cta = (it) => (it.status === 'known' ? 'Review' : it.status === 'partial' ? 'Keep going' : 'Start');

	// Group lessons by CEFR level, preserving the (id-ordered) sequence.
	let grouped = $derived.by(() => {
		const out = [];
		for (const it of items) {
			let g = out.find((x) => x.level === it.level);
			if (!g) {
				g = { level: it.level || 'A1', items: [] };
				out.push(g);
			}
			g.items.push(it);
		}
		return out;
	});
</script>

<div class="page-head">
	<h1>Grammar</h1>
	<p>Learn the rules, then drill them. Grammar is its own track — you apply each rule to words you
		already know, and some lessons unlock once you've started the ones they build on.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else}
	{#each grouped as group (group.level)}
		<div class="level-head"><span class="level-badge">{group.level}</span></div>
		<ol class="list">
			{#each group.items as it (it.id)}
				<li class="node {it.state}">
					<div class="dot">{dot(it.state)}</div>
					<div class="card">
						<div class="top">
							<span class="title">{it.title}</span>
							<span class="status {it.status}">{it.status}</span>
						</div>
						{#if it.explanation}<div class="expl">{it.explanation}</div>{/if}
						<div class="actions">
							{#if it.state === 'locked'}
								<span class="muted">Start {it.locked_on.join(', ')} first</span>
							{:else}
								<button class:primary={it.state !== 'done'} onclick={() => onOpen(it.id)}>
									{cta(it)} →
								</button>
								<span class="muted">{it.drill_count} drills</span>
							{/if}
						</div>
					</div>
				</li>
			{/each}
		</ol>
	{/each}
{/if}

<style>
	.level-head {
		margin: 1.6rem 0 0.8rem;
	}
	.level-head:first-child {
		margin-top: 0;
	}
	.level-badge {
		font-size: 0.85rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		padding: 0.2rem 0.7rem;
		border-radius: 999px;
		background: var(--accent);
		color: #04201d;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.node {
		display: grid;
		grid-template-columns: 40px 1fr;
		gap: 1rem;
		align-items: start;
	}
	.dot {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		background: var(--panel-2);
		border: 2px solid var(--border);
	}
	.node.active .dot {
		border-color: var(--accent);
		color: var(--accent);
	}
	.node.done .dot {
		background: var(--accent);
		border-color: var(--accent);
		color: #04201d;
	}
	.node.locked {
		opacity: 0.6;
	}
	.card {
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1rem 1.2rem;
	}
	.node.active .card {
		border-color: var(--accent);
	}
	.top {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
	}
	.title {
		font-size: 1.1rem;
		font-weight: 600;
	}
	.status {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted);
	}
	.status.known {
		color: var(--known);
	}
	.status.partial {
		color: var(--partial);
	}
	.expl {
		color: var(--muted);
		font-size: 0.9rem;
		margin-top: 0.3rem;
	}
	.actions {
		margin-top: 0.9rem;
		display: flex;
		align-items: center;
		gap: 0.8rem;
	}
</style>
