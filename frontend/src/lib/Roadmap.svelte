<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let { onOpen } = $props();

	let units = $state([]);
	let error = $state('');
	let loading = $state(true);

	async function load() {
		loading = true;
		error = '';
		try {
			units = await api.roadmap();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	onMount(load);

	const cta = (u) =>
		u.state === 'done' ? 'Review' : u.percent > 0 ? 'Continue' : 'Start';
</script>

<div class="page-head">
	<h1>Your roadmap</h1>
	<p>Work through the units in order. Each one teaches a small set of words and a bit of grammar; your
		progress fills in as you learn.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else}
	<ol class="path">
		{#each units as u, i (u.id)}
			<li class="node {u.state}">
				<div class="rail" class:first={i === 0} class:last={i === units.length - 1}></div>
				<div class="dot">
					{#if u.state === 'done'}✓{:else if u.state === 'locked'}🔒{:else}{u.percent}%{/if}
				</div>
				<div class="unit-card">
					<div class="unit-top">
						<span class="unit-title">{u.title}</span>
						<span class="unit-count">{u.known}/{u.target_total} words</span>
					</div>
					<div class="unit-desc">{u.description}</div>
					<div class="bar" style="margin-top: 0.7rem;">
						<span class="known" style="flex-basis: {u.percent}%"></span>
						<span class="unknown" style="flex-basis: {100 - u.percent}%"></span>
					</div>
					<div class="unit-actions">
						{#if u.state === 'locked'}
							<span class="muted">Finish the previous unit to unlock</span>
						{:else}
							<button class:primary={u.state !== 'done'} onclick={() => onOpen(u.id)}>
								{cta(u)} →
							</button>
						{/if}
					</div>
				</div>
			</li>
		{/each}
	</ol>
{/if}

<style>
	.path {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.node {
		position: relative;
		display: grid;
		grid-template-columns: 48px 1fr;
		gap: 1rem;
		padding-bottom: 1.1rem;
	}
	.rail {
		position: absolute;
		left: 23px;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--border);
	}
	.rail.first {
		top: 24px;
	}
	.rail.last {
		bottom: calc(100% - 24px);
	}
	.dot {
		position: relative;
		z-index: 1;
		width: 48px;
		height: 48px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.8rem;
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
	.unit-card {
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1.1rem 1.3rem;
	}
	.node.active .unit-card {
		border-color: var(--accent);
	}
	.unit-top {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
	}
	.unit-title {
		font-size: 1.1rem;
		font-weight: 600;
	}
	.unit-count {
		font-size: 0.78rem;
		color: var(--muted);
	}
	.unit-desc {
		color: var(--muted);
		font-size: 0.9rem;
		margin-top: 0.2rem;
	}
	.unit-actions {
		margin-top: 0.9rem;
	}
</style>
