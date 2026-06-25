<script lang="ts">
	import '../app.css';
	import { ModeWatcher } from 'mode-watcher';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { authState, checkSession } from '$lib/auth.svelte';
	import type { ThemeMode } from '$lib/types';
	import { appConfig } from '$lib/config.svelte';

	let { children } = $props();

	const defaultMode = $derived('system' as ThemeMode);
	const isLoginPage = $derived($page.url.pathname === '/login');

	onMount(() => {
		void (async () => {
			await checkSession();

			if (authState.authEnabled && !authState.authenticated && !isLoginPage) {
				await goto('/login');
				return;
			}

			if (isLoginPage && authState.authenticated) {
				await goto('/');
			}
		})();
	});

	$effect(() => {
		if (!authState.checked) return;

		if (authState.authEnabled && !authState.authenticated && $page.url.pathname !== '/login') {
			void goto('/login');
		}
	});
</script>

<ModeWatcher defaultMode={defaultMode} />

<svelte:head>
	<title>{appConfig.data?.title ?? 'GopDash'}</title>
</svelte:head>

{@render children()}
