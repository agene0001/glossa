<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import { speak } from '$lib/audio.js';

	let { unitId, live = false, lang = 'es', onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);
	let selected = $state(null);
	let revealed = $state(new Set());
	let practice = $state(null);
	let practiceLoading = $state(false);
	let practiceRevealed = $state(false);
	let saving = $state(false);
	let result = $state(null); // LessonResult → celebration

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

	const plain = (tokens) => tokens.map((t) => t.text).join('');

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
			result = await api.completeUnitLesson(unitId, true);
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

{#if result}
	<div class="card celebrate">
		<div class="emoji">🎉</div>
		<h2>Lesson complete!</h2>
		<p>
			{result.newly_known} new word{result.newly_known === 1 ? '' : 's'} learned ·
			{result.percent}% of this unit
		</p>
		{#if result.done}<p class="muted">Unit complete — the next one is unlocked.</p>{/if}
		{#if result.streak > 0}<p class="streak-big">🔥 {result.streak}-day streak</p>{/if}
		<button class="primary" onclick={onBack}>Back to roadmap →</button>
	</div>
{:else}
	<button class="link" onclick={onBack}>← Back to roadmap</button>

	{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if lesson}
		<div class="page-head" style="margin-top: 0.8rem;">
			<h1>{lesson.title}</h1>
			<p>{lesson.description}</p>
		</div>

		{#if lesson.grammar_tip}
			<div class="tip">
				<span class="tip-label">Grammar tip{#if lesson.grammar} · {lesson.grammar}{/if}</span>
				<p>{lesson.grammar_tip}</p>
			</div>
		{/if}

		<div class="card">
			<div class="lesson-examples">
				{#each lesson.examples as ex, i (i)}
					<div class="ex">
						<div class="story">{@render tline(ex.tokens)}</div>
						{#if revealed.has(i)}<div class="ex-tr">{ex.translation}</div>{/if}
						<div class="ex-actions">
							<button class="iconbtn" title="Listen" onclick={() => speak(plain(ex.tokens), lang)}>🔊</button>
							<button class="link" onclick={() => toggleReveal(i)}>
								{revealed.has(i) ? 'Hide' : 'Show'} translation
							</button>
						</div>
					</div>
				{/each}
			</div>

			{#if selected}
				<div class="meaning">
					<div>
						<button class="iconbtn" title="Listen" onclick={() => speak(selected.text, lang)}>🔊</button>
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
				<span class="muted">· tap a word for its meaning, 🔊 to hear it</span>
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
					<button
						class="vchip {w.status}"
						title="Listen"
						onclick={() => speak(w.lemma, lang)}>
						<strong>{w.lemma}</strong>{#if w.gloss} — {w.gloss}{/if} <span class="spk">🔊</span>
					</button>
				{/each}
			</div>
		</div>

		{#if practice}
			<div class="card" style="margin-top: 1.2rem;">
				<div class="nw-label">Practice sentence</div>
				<div class="story">{@render tline(practice.tokens)}</div>
				<div class="ex-actions">
					<button class="iconbtn" title="Listen" onclick={() => speak(plain(practice.tokens), lang)}>🔊</button>
					{#if practice.translation}
						<button class="link" onclick={() => (practiceRevealed = !practiceRevealed)}>
							{practiceRevealed ? 'Hide' : 'Reveal'} translation
						</button>
					{/if}
				</div>
				{#if practiceRevealed && practice.translation}<div class="ex-tr">{practice.translation}</div>{/if}
				<div class="row" style="margin-top: 1rem;">
					<button onclick={getPractice} disabled={practiceLoading}>Another</button>
				</div>
			</div>
		{/if}
	{/if}
{/if}
