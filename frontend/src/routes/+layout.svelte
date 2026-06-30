<script>
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api.js';

	let { children } = $props();

	let status = $state(null);
	let statusError = $state(false);
	let langs = $state([]);

	async function changeLang(e) {
		const code = e.currentTarget.value;
		try {
			await api.setTargetLanguage(code);
			location.reload(); // simplest reliable way to re-fetch everything
		} catch {
			/* ignore */
		}
	}

	const links = [
		{ href: '/', label: 'Learn' },
		{ href: '/vocab', label: 'Vocab' },
		{ href: '/grammar', label: 'Grammar' },
		{ href: '/sounds', label: 'Sounds' },
		{ href: '/quiz', label: 'Quiz' },
		{ href: '/review', label: 'Review' },
		{ href: '/chat', label: 'Chat', tag: 'Phase 2' },
		{ href: '/stats', label: 'Stats' }
	];

	const isActive = (href, path) => (href === '/' ? path === '/' : path.startsWith(href));

	onMount(async () => {
		try {
			status = await api.backendStatus();
		} catch {
			statusError = true;
		}
		try {
			langs = await api.availableLanguages();
		} catch {
			/* ignore */
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
				<div class="srow">
					Engine:
					{#if status.generator === 'anthropic'}
						<span class="badge live">live</span>
					{:else}
						<span class="badge mock">mock</span>
					{/if}
				</div>
				<label class="lang-row">
					<span>Language</span>
					<select onchange={changeLang} value={status.language}>
						{#each langs as l (l.code)}
							<option value={l.code}>{l.name}</option>
						{/each}
					</select>
				</label>
				{#if status.streak > 0}
					<div class="streak">🔥 {status.streak}-day streak</div>
				{/if}
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
