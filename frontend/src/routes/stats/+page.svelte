<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let s = $state(null);
	let error = $state('');
	let loading = $state(true);

	async function load() {
		loading = true;
		error = '';
		try {
			s = await api.stats();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	const pct = (part, total) => (total ? (part / total) * 100 : 0);
	// Heatmap intensity tier (0–4) from a day's activity count.
	function tier(c) {
		if (c === 0) return 0;
		if (c <= 2) return 1;
		if (c <= 4) return 2;
		if (c <= 7) return 3;
		return 4;
	}
	const tracks = $derived(
		s
			? [
					{ label: 'Course', percent: s.course_percent },
					{ label: 'Vocabulary', percent: s.vocab_percent },
					{ label: 'Grammar', percent: s.grammar_percent }
				]
			: []
	);
</script>

<div class="page-head">
	<h1>Stats</h1>
	<p>Your progress at a glance — mastery, accuracy, and how consistently you've shown up.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if s}
	<div class="cards">
		<div class="stat"><div class="n">🔥 {s.streak}</div><div class="l">day streak</div></div>
		<div class="stat"><div class="n">{s.active_days}</div><div class="l">active days</div></div>
		<div class="stat"><div class="n">{s.total_exercises}</div><div class="l">exercises done</div></div>
		<div class="stat"><div class="n">{s.total_exercises ? s.accuracy + '%' : '—'}</div><div class="l">accuracy</div></div>
	</div>

	<div class="card">
		<h2>Progress by track</h2>
		{#each tracks as t (t.label)}
			<div class="track">
				<div class="track-top"><span>{t.label}</span><span class="muted">{t.percent}%</span></div>
				<div class="bar"><span class="known" style="flex-basis: {t.percent}%"></span><span class="unknown" style="flex-basis: {100 - t.percent}%"></span></div>
			</div>
		{/each}
	</div>

	<div class="card">
		<h2>Vocabulary</h2>
		<div class="stat-grid">
			<div class="stat sm"><div class="n">{s.words_known}</div><div class="l">known</div></div>
			<div class="stat sm"><div class="n">{s.words_partial}</div><div class="l">learning</div></div>
			<div class="stat sm"><div class="n">{s.words_unknown}</div><div class="l">not yet seen</div></div>
			<div class="stat sm"><div class="n">{s.custom_words}</div><div class="l">your own words</div></div>
		</div>
		<div class="bar" title="{s.words_known} known / {s.words_partial} learning / {s.words_unknown} unseen">
			<span class="known" style="flex-basis: {pct(s.words_known, s.total_words)}%"></span>
			<span class="partial" style="flex-basis: {pct(s.words_partial, s.total_words)}%"></span>
			<span class="unknown" style="flex-basis: {pct(s.words_unknown, s.total_words)}%"></span>
		</div>
		<p class="muted small">Grammar — {s.grammar_known} known · {s.grammar_partial} learning · of {s.grammar_total} patterns</p>
	</div>

	<div class="card">
		<h2>Activity</h2>
		<p class="muted small">The last 12 weeks — darker means a busier day.</p>
		<div class="heatmap">
			{#each s.activity as d (d.date)}
				<div class="cell t{tier(d.count)}" title="{d.date}: {d.count} {d.count === 1 ? 'event' : 'events'}"></div>
			{/each}
		</div>
	</div>
{/if}

<style>
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
		gap: 1rem;
		margin-bottom: 1.4rem;
	}
	.stat {
		background: var(--panel);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1.1rem;
		text-align: center;
	}
	.stat .n {
		font-size: 1.7rem;
		font-weight: 700;
	}
	.stat .l {
		color: var(--muted);
		font-size: 0.82rem;
		margin-top: 0.2rem;
	}
	.stat.sm .n {
		font-size: 1.3rem;
	}
	.card {
		margin-top: 1.2rem;
	}
	.card h2 {
		margin: 0 0 0.9rem;
		font-size: 1.1rem;
	}
	.track {
		margin-bottom: 0.9rem;
	}
	.track-top {
		display: flex;
		justify-content: space-between;
		font-size: 0.9rem;
		margin-bottom: 0.3rem;
	}
	.stat-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.8rem;
		margin-bottom: 1rem;
	}
	@media (max-width: 560px) {
		.stat-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	.small {
		font-size: 0.82rem;
		margin-top: 0.8rem;
	}
	.heatmap {
		display: grid;
		grid-template-rows: repeat(7, 1fr);
		grid-auto-flow: column;
		grid-auto-columns: 1fr;
		gap: 3px;
		max-width: 100%;
	}
	.cell {
		aspect-ratio: 1;
		border-radius: 3px;
		background: var(--panel-2);
		border: 1px solid var(--border);
	}
	.cell.t1 {
		background: color-mix(in srgb, var(--accent) 30%, var(--panel-2));
	}
	.cell.t2 {
		background: color-mix(in srgb, var(--accent) 55%, var(--panel-2));
	}
	.cell.t3 {
		background: color-mix(in srgb, var(--accent) 80%, var(--panel-2));
	}
	.cell.t4 {
		background: var(--accent);
	}
</style>
