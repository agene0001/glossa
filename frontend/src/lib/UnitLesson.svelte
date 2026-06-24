<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';

	let { unitId, live = false, onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let selected = $state(null); // a tapped token
	let revealed = $state(new Set()); // example indices showing translation
	let practice = $state(null);
	let practiceLoading = $state(false);
	let practiceRevealed = $state(false);
	let saving = $state(false);

	async function load() {
		loading = true;
		error = '';
		try {
			lesson = await api.unitLesson(unitId);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);

	function pick(t) {
		if (!t.is_word || t.status == null) return;
		selected = selected === t ? null : t;
	}
	async function markKnown(t) {
		if (!t || t.lexeme_id == null) return;
		try {
			await api.setLexemeStatus(t.lexeme_id, 'known');
			t.status = 'known';
			selected = null;
		} catch (e) {
			error = String(e);
		}
	}
	function toggleReveal(i) {
		if (revealed.has(i)) revealed.delete(i);
		else revealed.add(i);
		revealed = new Set(revealed);
	}
	async function studied() {
		saving = true;
		try {
			await api.completeUnitLesson(unitId, true);
			onBack();
		} catch (e) {
			error = String(e);
			saving = false;
		}
	}
	async function getPractice() {
		practiceLoading = true;
		practiceRevealed = false;
		try {
			practice = await api.nextContentForUnit(unitId);
		} catch (e) {
			error = String(e);
		} finally {
			practiceLoading = false;
		}
	}
</script>

{#snippet tline(tokens)}
	{#each tokens as t, i (i)}
		{#if t.is_word}<span
				class="tok word {t.status}"
				role="button"
				tabindex="0"
				title={t.gloss ?? ''}
				onclick={() => pick(t)}
				onkeydown={(e) => e.key === 'Enter' && pick(t)}>{t.text}</span
			>{:else}<span class="tok">{t.text}</span>{/if}
	{/each}
{/snippet}

<button class="link" onclick={onBack}>← Back to roadmap</button>

{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if lesson}
	<div class="page-head" style="margin-top: 0.8rem;">
		<h1>{lesson.title}</h1>
		<p>{lesson.description}{#if lesson.grammar} · grammar focus: <strong>{lesson.grammar}</strong>{/if}</p>
	</div>

	<div class="card">
		<div class="lesson-examples">
			{#each lesson.examples as ex, i (i)}
				<div class="ex">
					<div class="story">{@render tline(ex.tokens)}</div>
					{#if revealed.has(i)}<div class="ex-tr">{ex.translation}</div>{/if}
					<button class="link" onclick={() => toggleReveal(i)}>
						{revealed.has(i) ? 'Hide' : 'Show'} translation
					</button>
				</div>
			{/each}
		</div>

		{#if selected}
			<div class="meaning">
				<div>
					<span class="m-word">{selected.text}</span>
					<span class="m-gloss">{selected.gloss ?? 'no meaning on file'}</span>
				</div>
				{#if selected.lexeme_id != null && selected.status !== 'known'}
					<button onclick={() => markKnown(selected)}>✓ I know this word</button>
				{/if}
			</div>
		{/if}

		<div class="legend">
			<span><span class="swatch" style="background: var(--known)"></span>known</span>
			<span><span class="swatch" style="background: var(--partial)"></span>learning</span>
			<span><span class="swatch" style="background: var(--new)"></span>new</span>
			<span class="muted">· tap a word for its meaning</span>
		</div>

		<div class="row" style="margin-top: 1.4rem;">
			<button class="primary" onclick={studied} disabled={saving}>I've studied this ✓</button>
			{#if live}
				<button onclick={getPractice} disabled={practiceLoading}>Extra AI practice</button>
			{/if}
		</div>
	</div>

	<div class="card" style="margin-top: 1.2rem;">
		<div class="nw-label">Words in this unit</div>
		<div class="vocab">
			{#each lesson.words as w (w.lexeme_id)}
				<span class="vchip {w.status}"
					><strong>{w.lemma}</strong>{#if w.gloss} — {w.gloss}{/if}</span>
			{/each}
		</div>
	</div>

	{#if practice}
		<div class="card" style="margin-top: 1.2rem;">
			<div class="nw-label">Practice sentence</div>
			<div class="story">{@render tline(practice.tokens)}</div>
			{#if practice.translation}
				{#if practiceRevealed}<div class="ex-tr">{practice.translation}</div>{/if}
				<button class="link" onclick={() => (practiceRevealed = !practiceRevealed)}>
					{practiceRevealed ? 'Hide' : 'Reveal'} translation
				</button>
			{/if}
			<div class="row" style="margin-top: 1rem;">
				<button onclick={getPractice} disabled={practiceLoading}>Another</button>
			</div>
		</div>
	{/if}
{/if}
