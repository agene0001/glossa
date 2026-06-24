<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api.js';
	import Onboarding from '$lib/Onboarding.svelte';
	import Roadmap from '$lib/Roadmap.svelte';
	import UnitLesson from '$lib/UnitLesson.svelte';

	let booting = $state(true);
	let showOnboarding = $state(false);
	let live = $state(false);
	let selectedUnit = $state(null);

	onMount(async () => {
		try {
			const s = await api.backendStatus();
			live = s.generator === 'anthropic';
		} catch {
			/* ignore */
		}
		let onboarded = false;
		try {
			onboarded = localStorage.getItem('glossa_onboarded') === '1';
		} catch {
			/* ignore */
		}
		// Existing learners with progress skip onboarding.
		if (!onboarded) {
			try {
				const ov = await api.graphOverview(1);
				if (ov.known + ov.partial > 0) {
					localStorage.setItem('glossa_onboarded', '1');
					onboarded = true;
				}
			} catch {
				/* treat as new */
			}
		}
		booting = false;
		showOnboarding = !onboarded;
	});

	const onboardingDone = () => (showOnboarding = false);
	const openUnit = (id) => (selectedUnit = id);
	const backToRoadmap = () => (selectedUnit = null);
</script>

{#if booting}
	<p class="muted">Loading…</p>
{:else if showOnboarding}
	<Onboarding onDone={onboardingDone} />
{:else if selectedUnit !== null}
	{#key selectedUnit}
		<UnitLesson unitId={selectedUnit} {live} onBack={backToRoadmap} />
	{/key}
{:else}
	<Roadmap onOpen={openUnit} />
{/if}
