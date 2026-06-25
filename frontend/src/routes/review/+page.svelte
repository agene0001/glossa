<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api.js';

	let ov = $state(null);
	let reviewCount = $state(0);
	let error = $state('');
	let busy = $state(false);

	async function load() {
		error = '';
		try {
			ov = await api.graphOverview();
		} catch (e) {
			error = String(e);
		}
		try {
			reviewCount = await api.reviewableCount();
		} catch {
			/* ignore */
		}
	}

	async function know(item) {
		busy = true;
		try {
			await api.setLexemeStatus(item.lexeme_id, 'known');
			await load();
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}

	const pct = (part, total) => (total ? (part / total) * 100 : 0);

	onMount(load);
</script>

<div class="page-head">
	<h1>Review</h1>
	<p>Your knowledge graph: what you know, and what's queued next — and why.</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if ov}
	<div class="card">
		<div class="stat-grid">
			<div class="stat"><div class="n">{ov.known}</div><div class="l">known words</div></div>
			<div class="stat"><div class="n">{ov.partial}</div><div class="l">learning</div></div>
			<div class="stat"><div class="n">{ov.unknown}</div><div class="l">not yet seen</div></div>
			<div class="stat"><div class="n">{ov.total_lexemes}</div><div class="l">total in list</div></div>
		</div>

		<div class="bar" title="{ov.known} known / {ov.partial} learning / {ov.unknown} unseen">
			<span class="known" style="flex-basis: {pct(ov.known, ov.total_lexemes)}%"></span>
			<span class="partial" style="flex-basis: {pct(ov.partial, ov.total_lexemes)}%"></span>
			<span class="unknown" style="flex-basis: {pct(ov.unknown, ov.total_lexemes)}%"></span>
		</div>

		<p class="muted" style="margin-top: 1rem;">
			Grammar patterns — {ov.grammar_known} known · {ov.grammar_partial} learning ·
			{ov.grammar_unknown} unseen
		</p>
	</div>

	{#if reviewCount > 0}
		<div class="card review-cta">
			<div>
				<strong>{reviewCount}</strong> word{reviewCount === 1 ? '' : 's'} ready to review
				<div class="muted">Test yourself with a quick quiz — weakest words first.</div>
			</div>
			<button class="primary" onclick={() => goto('/quiz')}>Start review →</button>
		</div>
	{/if}

	<div class="card" style="margin-top: 1.4rem;">
		<div class="row" style="justify-content: space-between;">
			<h2 style="margin: 0; font-size: 1.1rem;">Up next</h2>
			<button onclick={load} disabled={busy}>Refresh</button>
		</div>
		<p class="muted" style="margin-top: 0.3rem;">
			The most frequent words you haven't met. Mark ones you already know to skip them.
		</p>

		{#if ov.next_queue.length}
			<ul class="queue" style="margin-top: 1rem;">
				{#each ov.next_queue as item (item.lexeme_id)}
					<li>
						<span class="lemma">{item.lemma}</span>
						{#if item.gloss}<span class="gloss">{item.gloss}</span>{/if}
						<span class="pos">{item.pos}</span>
						<span class="reason">{item.reason}</span>
						<button onclick={() => know(item)} disabled={busy}>I know this</button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="muted" style="margin-top: 1rem;">Nothing queued — you've seen the whole list! 🎉</p>
		{/if}
	</div>
{:else if !error}
	<p class="muted">Loading…</p>
{/if}
