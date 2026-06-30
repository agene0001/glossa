<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import StudyQuiz from '$lib/StudyQuiz.svelte';

	let { packId, lang = 'es', onBack } = $props();

	let lesson = $state(null);
	let error = $state('');
	let loading = $state(true);

	async function load() {
		loading = true;
		error = '';
		try {
			lesson = await api.packLesson(packId);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
	onMount(load);
</script>

<button class="link" onclick={onBack}>← All packs</button>

{#if error}<div class="error" style="margin-top: 0.6rem;">{error}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if lesson}
	<div class="page-head" style="margin-top: 0.8rem;">
		<h1><span class="emoji">{lesson.emoji}</span> {lesson.title}</h1>
		<p>{lesson.description}</p>
	</div>

	<StudyQuiz {lesson} {lang} loadQuiz={() => api.packQuiz(packId, 12)} onExit={onBack} exitLabel="Back to packs →" />
{/if}

<style>
	.emoji {
		font-size: 1.4rem;
	}
</style>
