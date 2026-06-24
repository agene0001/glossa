<script>
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api.js';

	let { children } = $props();

	let status = $state(null);
	let statusError = $state(false);

	const links = [
		{ href: '/', label: 'Reading' },
		{ href: '/review', label: 'Review' },
		{ href: '/chat', label: 'Chat', tag: 'Phase 2' },
		{ href: '/stats', label: 'Stats', tag: 'soon' }
	];

	const isActive = (href, path) => (href === '/' ? path === '/' : path.startsWith(href));

	onMount(async () => {
		try {
			status = await api.backendStatus();
		} catch {
			statusError = true;
		}
	});
</script>

<div class="app">
	<aside class="sidebar">
		<div class="brand"><span class="dot"></span> Glossa</div>
		{#each links as l (l.href)}
			<a class="nav-link {isActive(l.href, $page.url.pathname) ? 'active' : ''}" href={l.href}>
				{l.label}{#if l.tag}<span class="tag">{l.tag}</span>{/if}
			</a>
		{/each}
		<div class="spacer"></div>
		<div class="status">
			{#if status}
				Engine:
				{#if status.generator === 'anthropic'}
					<span class="badge live">live</span>
				{:else}
					<span class="badge mock">mock</span>
				{/if}
				<br />Target: <strong>{status.language}</strong>
			{:else if statusError}
				<span class="muted">backend offline — open via <code>npm run dev</code></span>
			{:else}
				<span class="muted">connecting…</span>
			{/if}
		</div>
	</aside>
	<main class="content">
		{@render children()}
	</main>
</div>
