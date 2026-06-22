<script lang="ts">
	import { goto } from '$app/navigation';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { login } from '$lib/auth.svelte';
	import { browserLocale, t } from '$lib/locale';

	let username = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let loading = $state(false);

	const locale = $derived(browserLocale());

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = null;
		loading = true;

		try {
			await login(username.trim(), password);
			await goto('/');
		} catch (err) {
			error = err instanceof Error ? err.message : t('login_error', locale);
		} finally {
			loading = false;
		}
	}
</script>

<div class="min-h-screen flex items-center justify-center px-4 bg-background">
	<Card class="w-full max-w-sm">
		<CardHeader class="text-center space-y-3">
			<div class="mx-auto flex size-12 items-center justify-center rounded-xl bg-primary/10 text-primary">
				<LayoutDashboardIcon class="size-6" />
			</div>
			<CardTitle class="text-xl">Gopdash</CardTitle>
			<p class="text-sm text-muted-foreground">{t('login_subtitle', locale)}</p>
		</CardHeader>
		<CardContent>
			<form class="space-y-4" onsubmit={handleSubmit}>
				<div class="space-y-2">
					<label for="username" class="text-sm font-medium">{t('login_username', locale)}</label>
					<input
						id="username"
						name="username"
						type="text"
						autocomplete="username"
						bind:value={username}
						required
						class="flex h-9 w-full rounded-lg border border-input bg-background px-3 text-sm shadow-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
					/>
				</div>

				<div class="space-y-2">
					<label for="password" class="text-sm font-medium">{t('login_password', locale)}</label>
					<input
						id="password"
						name="password"
						type="password"
						autocomplete="current-password"
						bind:value={password}
						required
						class="flex h-9 w-full rounded-lg border border-input bg-background px-3 text-sm shadow-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
					/>
				</div>

				{#if error}
					<p class="text-sm text-destructive">{error}</p>
				{/if}

				<Button type="submit" class="w-full" disabled={loading}>
					{loading ? t('login_submitting', locale) : t('login_submit', locale)}
				</Button>
			</form>
		</CardContent>
	</Card>
</div>
