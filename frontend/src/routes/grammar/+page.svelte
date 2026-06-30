<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import GrammarTrack from '$lib/GrammarTrack.svelte';
	import GrammarLesson from '$lib/GrammarLesson.svelte';

	let lang = $state('es');
	let selected = $state(null);

	onMount(async () => {
		try {
			const s = await api.backendStatus();
			lang = s.language || 'es';
		} catch {
			/* ignore */
		}
	});

	const open = (id) => (selected = id);
	const back = () => (selected = null);
</script>

{#if selected !== null}
	{#key selected}
		<GrammarLesson patternId={selected} {lang} onBack={back} />
	{/key}
{:else}
	<GrammarTrack onOpen={open} />
{/if}
