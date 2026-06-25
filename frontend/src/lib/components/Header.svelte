<script lang="ts">
	import ActivityIcon from '@lucide/svelte/icons/activity';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import { goto } from '$app/navigation';
	import { appConfig } from '$lib/config.svelte';
	import { authState, logout } from '$lib/auth.svelte';
	import { liveData } from '$lib/sse.svelte';
	import { initLayoutLock, layoutState } from '$lib/layout.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import ThemeToggle from './ThemeToggle.svelte';
	import LayoutLockToggle from './LayoutLockToggle.svelte';
	import AppLogo from './AppLogo.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { onMount } from 'svelte';

	onMount(() => {
		initLayoutLock();
	});

	async function handleLogout() {
		await logout();
		await goto('/login');
	}

	const locale = $derived(resolveLocale(appConfig.data));
</script>

<header class="border-b bg-card/50 backdrop-blur-sm sticky top-0 z-50">
	{#if layoutState.savedNotice}
		<div
			class="pointer-events-none absolute inset-x-0 top-1/2 z-10 flex -translate-y-1/2 justify-center px-4"
			aria-live="polite"
		>
			<span class="rounded-md bg-card/95 px-2.5 py-1 text-xs text-muted-foreground shadow-sm ring-1 ring-border">
				{t('dashboard_layout_saved', locale)}
			</span>
		</div>
	{/if}
	<div class="container mx-auto px-4 h-14 flex items-center justify-between">
		<div class="flex items-center gap-3">
			<AppLogo class="size-8 rounded-md" />
			<h1 class="text-lg font-bold tracking-tight">
				{appConfig.data?.title ?? 'GopDash'}
			</h1>
			<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
				<ActivityIcon class="size-3 {liveData.sseConnected ? 'text-green-500' : ''}" />
				<span>{liveData.sseConnected ? t('header_live', locale) : t('header_connecting', locale)}</span>
			</div>
		</div>

		<div class="flex items-center gap-2">
			<LayoutLockToggle />
			<ThemeToggle />
			{#if authState.authEnabled && authState.authenticated}
				<Button variant="outline" size="icon" onclick={handleLogout} title={t('logout', locale)}>
					<LogOutIcon class="size-4" />
					<span class="sr-only">{t('logout', locale)}</span>
				</Button>
			{/if}
		</div>
	</div>
</header>
