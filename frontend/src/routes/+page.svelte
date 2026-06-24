<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import Onboarding from '$lib/Onboarding.svelte';

	let showOnboarding = $state(false);
	let booting = $state(true);

	let content = $state(null);
	let loading = $state(false);
	let error = $state('');
	let selectedIndex = $state(-1);
	let showTranslation = $state(false);

	let selectedTok = $derived(
		content && selectedIndex >= 0 ? content.tokens[selectedIndex] : null
	);

	async function load() {
		loading = true;
		error = '';
		selectedIndex = -1;
		showTranslation = false;
		try {
			content = await api.nextContent();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function finish(understood) {
		if (!content) return;
		try {
			await api.recordStoryRead(content.story_id, understood);
		} catch (e) {
			error = String(e);
			return;
		}
		await load();
	}

	// Tap a word to see its meaning (not to mark it — that's a separate action).
	function selectWord(i, tok) {
		if (!tok.is_word || tok.status == null) return;
		selectedIndex = selectedIndex === i ? -1 : i;
	}

	async function markKnown(tok) {
		if (!tok || tok.lexeme_id == null) return;
		try {
			await api.setLexemeStatus(tok.lexeme_id, 'known');
			tok.status = 'known';
			selectedIndex = -1;
		} catch (e) {
			error = String(e);
		}
	}

	function onboardingDone() {
		showOnboarding = false;
		load();
	}

	onMount(async () => {
		let onboarded = false;
		try {
			onboarded = localStorage.getItem('glossa_onboarded') === '1';
		} catch {
			/* ignore */
		}
		// Existing users with progress shouldn't see onboarding.
		if (!onboarded) {
			try {
				const ov = await api.graphOverview(1);
				if (ov.known + ov.partial > 0) {
					localStorage.setItem('glossa_onboarded', '1');
					onboarded = true;
				}
			} catch {
				/* ignore — treat as new */
			}
		}
		booting = false;
		if (onboarded) await load();
		else showOnboarding = true;
	});
</script>

{#if booting}
	<p class="muted">Loading…</p>
{:else if showOnboarding}
	<Onboarding onDone={onboardingDone} />
{:else}
	<div class="page-head">
		<h1>Reading</h1>
		<p>Read the sentence. Tap any word to see what it means; reveal the full translation if you
			need it.</p>
	</div>

	{#if error}<div class="error">{error}</div>{/if}

	<div class="card">
		{#if loading && !content}
			<p class="muted">Generating…</p>
		{:else if content}
			<div class="story">
				{#each content.tokens as tok, i (i)}
					{#if tok.is_word}
						<span
							class="tok word {tok.status} {selectedIndex === i ? 'sel' : ''}"
							role="button"
							tabindex="0"
							title={tok.gloss ?? ''}
							onclick={() => selectWord(i, tok)}
							onkeydown={(e) => e.key === 'Enter' && selectWord(i, tok)}>{tok.text}</span>
					{:else}<span class="tok">{tok.text}</span>{/if}
				{/each}
			</div>

			{#if selectedTok}
				<div class="meaning">
					<div>
						<span class="m-word">{selectedTok.text}</span>
						<span class="m-gloss">{selectedTok.gloss ?? 'no meaning on file'}</span>
					</div>
					{#if selectedTok.lexeme_id != null && selectedTok.status !== 'known'}
						<button onclick={() => markKnown(selectedTok)}>✓ I know this word</button>
					{/if}
				</div>
			{/if}

			{#if content.new_words.length}
				<div class="newwords">
					<div class="nw-label">New words</div>
					<div class="glossary">
						{#each content.new_words as w (w.lemma)}
							<span class="chip"
								><strong>{w.lemma}</strong>{#if w.gloss} — {w.gloss}{/if}</span>
						{/each}
					</div>
				</div>
			{/if}

			{#if content.translation}
				<div class="translation">
					{#if showTranslation}
						<div class="t-text">{content.translation}</div>
						<button class="link" onclick={() => (showTranslation = false)}>Hide translation</button>
					{:else}
						<button class="link" onclick={() => (showTranslation = true)}>Reveal translation</button>
					{/if}
				</div>
			{/if}

			<div class="legend">
				<span><span class="swatch" style="background: var(--known)"></span>known</span>
				<span><span class="swatch" style="background: var(--partial)"></span>learning</span>
				<span><span class="swatch" style="background: var(--new)"></span>new</span>
				<span><span class="swatch" style="background: var(--unknown)"></span>unknown</span>
				<span class="muted">· {Math.round(content.known_ratio * 100)}% known</span>
				{#if content.grammar_targeted}
					<span class="muted">· focus: {content.grammar_targeted}</span>
				{/if}
			</div>

			<div class="row" style="margin-top: 1.6rem;">
				<button class="primary" onclick={() => finish(true)} disabled={loading}>
					I understood it ✓
				</button>
				<button onclick={() => finish(false)} disabled={loading}>Too hard — skip</button>
			</div>
		{:else}
			<p class="muted">No content yet.</p>
		{/if}
	</div>
{/if}
