<script>
	import { api } from '$lib/api.js';

	let { onDone } = $props();

	let step = $state('choose'); // 'choose' | 'placement'
	let candidates = $state([]);
	let known = $state(new Set());
	let loading = $state(false);
	let error = $state('');

	async function startPlacement() {
		loading = true;
		error = '';
		try {
			const ov = await api.graphOverview(40);
			candidates = ov.next_queue;
			step = 'placement';
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function toggle(item) {
		const id = item.lexeme_id;
		const wasKnown = known.has(id);
		try {
			await api.setLexemeStatus(id, wasKnown ? 'unknown' : 'known');
			if (wasKnown) known.delete(id);
			else known.add(id);
			known = new Set(known); // trigger reactivity
		} catch (e) {
			error = String(e);
		}
	}

	function finish() {
		try {
			localStorage.setItem('glossa_onboarded', '1');
		} catch {
			/* ignore */
		}
		onDone();
	}
</script>

<div class="page-head">
	<h1>Welcome to Glossa</h1>
	<p>Glossa teaches by showing you sentences you can *almost* understand, then a few new words at a
		time. First, where are you starting from?</p>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if step === 'choose'}
	<div class="choose">
		<button class="choice" onclick={finish}>
			<div class="choice-title">I'm a complete beginner</div>
			<div class="choice-sub">Start from the most common words and build up.</div>
		</button>
		<button class="choice" onclick={startPlacement} disabled={loading}>
			<div class="choice-title">I already know some Spanish</div>
			<div class="choice-sub">Tick the words you know so Glossa starts at the right level.</div>
		</button>
	</div>
{:else}
	<div class="card">
		<p class="muted" style="margin-top: 0;">
			Tap every word you already know. {known.size} marked. You can always change these later in
			Review.
		</p>
		<div class="placement">
			{#each candidates as item (item.lexeme_id)}
				<button
					class="pchip {known.has(item.lexeme_id) ? 'on' : ''}"
					onclick={() => toggle(item)}>
					<span class="pl">{item.lemma}</span>
					{#if item.gloss}<span class="pg">{item.gloss}</span>{/if}
				</button>
			{/each}
		</div>
		<div class="row" style="margin-top: 1.4rem;">
			<button class="primary" onclick={finish}>Start reading →</button>
			<button onclick={finish}>Skip — I'll mark them as I go</button>
		</div>
	</div>
{/if}

<style>
	.choose {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}
	@media (max-width: 720px) {
		.choose {
			grid-template-columns: 1fr;
		}
	}
	.choice {
		text-align: left;
		padding: 1.6rem;
		border-radius: 14px;
		background: var(--panel);
	}
	.choice:hover {
		border-color: var(--accent);
	}
	.choice-title {
		font-size: 1.15rem;
		font-weight: 600;
		margin-bottom: 0.35rem;
	}
	.choice-sub {
		color: var(--muted);
		font-size: 0.9rem;
	}
	.placement {
		display: flex;
		flex-wrap: wrap;
		gap: 0.55rem;
		margin-top: 1rem;
	}
	.pchip {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1px;
		padding: 0.45rem 0.7rem;
		border-radius: 10px;
	}
	.pchip.on {
		background: var(--accent-soft);
		border-color: var(--accent);
	}
	.pchip .pl {
		font-weight: 600;
	}
	.pchip .pg {
		font-size: 0.72rem;
		color: var(--muted);
	}
</style>
