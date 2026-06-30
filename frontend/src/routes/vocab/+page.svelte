<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import VocabPacks from '$lib/VocabPacks.svelte';
	import VocabPack from '$lib/VocabPack.svelte';
	import Decks from '$lib/Decks.svelte';
	import Deck from '$lib/Deck.svelte';

	let lang = $state('es');
	let selected = $state(null); // { type: 'pack' | 'deck', id }

	onMount(async () => {
		try {
			const s = await api.backendStatus();
			lang = s.language || 'es';
		} catch {
			/* ignore */
		}
	});

	const openPack = (id) => (selected = { type: 'pack', id });
	const openDeck = (id) => (selected = { type: 'deck', id });
	const back = () => (selected = null);
</script>

{#if selected?.type === 'pack'}
	{#key selected.id}
		<VocabPack packId={selected.id} {lang} onBack={back} />
	{/key}
{:else if selected?.type === 'deck'}
	{#key selected.id}
		<Deck deckId={selected.id} {lang} onBack={back} />
	{/key}
{:else}
	<VocabPacks onOpen={openPack} />
	<Decks onOpen={openDeck} />
{/if}
