<script lang="ts">
	import type { WidgetConfig, JellyfinStatus, JellyfinNowPlayingSession } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { api } from '$lib/api';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import { Film, Pause, Play, Tv } from 'lucide-svelte';
	import { onDestroy, onMount } from 'svelte';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'jellyfin' }>;
	}

	let { widget }: Props = $props();
	let status = $state<JellyfinStatus | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	const locale = $derived(resolveLocale(appConfig.data));
	const refreshMs = $derived((appConfig.data?.refresh_interval ?? 30) * 1000);
	const configured = $derived(appConfig.data?.jellyfin_configured ?? false);
	const showNowPlaying = $derived(widget.show_now_playing ?? true);
	const showLibraryCounts = $derived(widget.show_library_counts ?? true);
	const maxSessions = $derived(widget.max_sessions ?? 3);

	let refreshTimer: ReturnType<typeof setInterval> | null = null;

	async function loadStatus() {
		if (!configured) {
			loading = false;
			return;
		}

		try {
			status = await api.getJellyfinStatus({
				showNowPlaying,
				showLibraryCounts,
				maxSessions
			});
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : t('jellyfin_error', locale);
		} finally {
			loading = false;
		}
	}

	function progressPercent(session: JellyfinNowPlayingSession): number {
		if (session.runtime_ticks <= 0) return 0;
		return Math.min(100, (session.position_ticks / session.runtime_ticks) * 100);
	}

	function sessionSubtitle(session: JellyfinNowPlayingSession): string {
		if (session.series_name) {
			const parts = [session.series_name];
			if (session.season_episode) parts.push(session.season_episode);
			return parts.join(' · ');
		}
		if (session.year) return String(session.year);
		return session.item_type;
	}

	function formatCount(value: number): string {
		return value.toLocaleString(locale);
	}

	onMount(() => {
		void loadStatus();
		refreshTimer = setInterval(() => void loadStatus(), refreshMs);
	});

	onDestroy(() => {
		if (refreshTimer) clearInterval(refreshTimer);
	});
</script>

<WidgetHeader {widget} title={widget.title ?? 'Jellyfin'} />
<CardContent>
	{#if !configured}
		<p class="text-xs text-muted-foreground">{t('jellyfin_not_configured', locale)}</p>
	{:else if loading}
		<p class="text-xs text-muted-foreground">{t('jellyfin_loading', locale)}</p>
	{:else if error}
		<p class="text-xs text-destructive">{error}</p>
	{:else if status}
		<div class="space-y-3">
			{#if showLibraryCounts && status.library_counts}
				<div class="grid grid-cols-3 gap-2 text-center">
					<div class="rounded-md bg-muted/50 px-2 py-1.5">
						<div class="text-[10px] uppercase tracking-wide text-muted-foreground">
							{t('jellyfin_movies', locale)}
						</div>
						<div class="text-sm font-semibold tabular-nums">
							{formatCount(status.library_counts.movies)}
						</div>
					</div>
					<div class="rounded-md bg-muted/50 px-2 py-1.5">
						<div class="text-[10px] uppercase tracking-wide text-muted-foreground">
							{t('jellyfin_series', locale)}
						</div>
						<div class="text-sm font-semibold tabular-nums">
							{formatCount(status.library_counts.series)}
						</div>
					</div>
					<div class="rounded-md bg-muted/50 px-2 py-1.5">
						<div class="text-[10px] uppercase tracking-wide text-muted-foreground">
							{t('jellyfin_episodes', locale)}
						</div>
						<div class="text-sm font-semibold tabular-nums">
							{formatCount(status.library_counts.episodes)}
						</div>
					</div>
				</div>
			{/if}

			{#if showNowPlaying}
				{#if status.now_playing.length === 0}
					<p class="text-xs text-muted-foreground">{t('jellyfin_nothing_playing', locale)}</p>
				{:else}
					<div class="space-y-2">
						<div class="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
							{t('jellyfin_now_playing', locale)}
						</div>
						{#each status.now_playing as session (session.item_id + session.user_name)}
							<div class="flex gap-2 rounded-md border border-border/60 p-2">
								<img
									src={api.jellyfinImageUrl(session.item_id)}
									alt=""
									class="size-12 shrink-0 rounded object-cover bg-muted"
									loading="lazy"
								/>
								<div class="min-w-0 flex-1 space-y-1">
									<div class="flex items-start gap-1">
										{#if session.item_type === 'Episode'}
											<Tv class="mt-0.5 size-3 shrink-0 text-muted-foreground" />
										{:else}
											<Film class="mt-0.5 size-3 shrink-0 text-muted-foreground" />
										{/if}
										<div class="min-w-0">
											<div class="truncate text-xs font-medium">{session.title}</div>
											<div class="truncate text-[10px] text-muted-foreground">
												{sessionSubtitle(session)}
											</div>
										</div>
										{#if session.is_paused}
											<Pause class="ml-auto size-3 shrink-0 text-muted-foreground" />
										{:else}
											<Play class="ml-auto size-3 shrink-0 text-primary" />
										{/if}
									</div>
									<div class="h-1 overflow-hidden rounded-full bg-muted">
										<div
											class="h-full rounded-full bg-primary transition-all"
											style:width="{progressPercent(session)}%"
										></div>
									</div>
									<div class="truncate text-[10px] text-muted-foreground">
										{session.user_name}
										{#if session.device_name}
											· {session.device_name}
										{/if}
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</CardContent>
