<script lang="ts">
	import { onMount } from 'svelte';
	import { connectSSE } from '$lib/sse.svelte';
	import { fetchConfig } from '$lib/config.svelte';
	import { authState, checkSession } from '$lib/auth.svelte';
	import Header from '$lib/components/Header.svelte';

	let { children } = $props();

	onMount(() => {
		let disconnectSSE: (() => void) | undefined;

		void (async () => {
			if (!authState.checked) {
				await checkSession();
			}
			if (authState.authEnabled && !authState.authenticated) return;

			await fetchConfig();
			disconnectSSE = connectSSE();
		})();

		return () => disconnectSSE?.();
	});
</script>

<div class="min-h-screen flex flex-col">
	<Header />
	<main class="flex-1 container mx-auto px-4 py-6">
		{@render children()}
	</main>
</div>
