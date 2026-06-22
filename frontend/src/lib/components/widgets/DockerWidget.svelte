<script lang="ts">
	import type { WidgetConfig, ContainerInfo } from '$lib/types';
	import { CardContent } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import WidgetHeader from '$lib/components/WidgetHeader.svelte';
	import { liveData } from '$lib/sse.svelte';
	import { api } from '$lib/api';
	import { formatBytes } from '$lib/utils';
	import { appConfig } from '$lib/config.svelte';
	import { resolveLocale, t } from '$lib/locale';
	import {
		aggregateCpu,
		aggregateGroupState,
		aggregateMemory,
		containerDisplayName,
		groupContainers,
		groupHasRunning,
		type ContainerGroup
	} from '$lib/docker-groups';
	import { Play, Square, RotateCw, ChevronDown, ChevronRight } from 'lucide-svelte';
	import { onMount } from 'svelte';

	interface Props {
		widget: Extract<WidgetConfig, { type: 'docker' }>;
	}

	let { widget }: Props = $props();
	let loading = $state<Record<string, boolean>>({});
	let expandedGroups = $state<Set<string>>(new Set());

	const groupBy = $derived(widget.group_by ?? 'flat');
	const collapseGroups = $derived(widget.collapse_groups ?? true);
	const locale = $derived(resolveLocale(appConfig.data));

	onMount(async () => {
		if (liveData.dockerContainers.length === 0) {
			try {
				const filter = widget.containers?.join(',');
				const data = await api.getContainers(filter, widget.show_all);
				liveData.dockerContainers = data;
			} catch (e) {
				console.error('Failed to load containers:', e);
			}
		}
	});

	let containers = $derived(
		liveData.dockerContainers.filter((c) => {
			if (widget.show_all) return true;
			if (!widget.containers?.length) return true;
			return widget.containers.some(
				(f) =>
					c.name.includes(f) ||
					c.compose_project === f ||
					(c.compose_project?.includes(f) ?? false)
			);
		})
	);

	let groups = $derived(groupContainers(containers, groupBy));

	function isExpanded(group: ContainerGroup): boolean {
		if (!group.isCompose) return true;
		if (!collapseGroups) return true;
		return expandedGroups.has(group.key);
	}

	function toggleGroup(key: string) {
		const next = new Set(expandedGroups);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expandedGroups = next;
	}

	async function runContainerAction(id: string, fn: 'start' | 'stop' | 'restart') {
		if (fn === 'start') await api.startContainer(id);
		else if (fn === 'stop') await api.stopContainer(id);
		else await api.restartContainer(id);
	}

	async function action(id: string, fn: 'start' | 'stop' | 'restart') {
		loading[id] = true;
		try {
			await runContainerAction(id, fn);
		} catch (e) {
			console.error(e);
		} finally {
			loading[id] = false;
		}
	}

	function groupLoadingKey(group: ContainerGroup): string {
		return `group:${group.key}`;
	}

	function isGroupLoading(group: ContainerGroup): boolean {
		return (
			!!loading[groupLoadingKey(group)] ||
			group.containers.some((c) => loading[c.id])
		);
	}

	async function groupAction(group: ContainerGroup, fn: 'start' | 'stop' | 'restart') {
		const gKey = groupLoadingKey(group);
		const targets = group.containers.filter((c) => {
			const running = c.state.toLowerCase() === 'running';
			if (fn === 'start') return !running;
			if (fn === 'stop') return running;
			return true;
		});
		if (targets.length === 0) return;

		loading[gKey] = true;
		for (const c of targets) loading[c.id] = true;

		try {
			await Promise.all(targets.map((c) => runContainerAction(c.id, fn)));
		} catch (e) {
			console.error(e);
		} finally {
			loading[gKey] = false;
			for (const c of targets) loading[c.id] = false;
		}
	}

	function groupHasStopped(group: ContainerGroup): boolean {
		return group.containers.some((c) => c.state.toLowerCase() !== 'running');
	}

	function stateColor(state: string): string {
		switch (state.toLowerCase()) {
			case 'running':
				return 'bg-green-500';
			case 'exited':
				return 'bg-red-500';
			case 'paused':
				return 'bg-yellow-500';
			case 'partial':
				return 'bg-yellow-500';
			default:
				return 'bg-gray-400';
		}
	}

	function runningLabel(group: ContainerGroup): string {
		const running = group.containers.filter((c) => c.state.toLowerCase() === 'running').length;
		return `${running}/${group.containers.length}`;
	}
</script>

{#snippet groupActions(group: ContainerGroup)}
	<div class="flex gap-1 shrink-0" role="group" aria-label="Actions {group.name}">
		{#if groupHasStopped(group)}
			<Button
				variant="ghost"
				size="icon"
				disabled={isGroupLoading(group)}
				onclick={(e) => {
					e.stopPropagation();
					void groupAction(group, 'start');
				}}
			>
				<Play class="w-3 h-3" />
			</Button>
		{/if}
		{#if groupHasRunning(group.containers)}
			<Button
				variant="ghost"
				size="icon"
				disabled={isGroupLoading(group)}
				onclick={(e) => {
					e.stopPropagation();
					void groupAction(group, 'stop');
				}}
			>
				<Square class="w-3 h-3" />
			</Button>
		{/if}
		<Button
			variant="ghost"
			size="icon"
			disabled={isGroupLoading(group)}
			onclick={(e) => {
				e.stopPropagation();
				void groupAction(group, 'restart');
			}}
		>
			<RotateCw class="w-3 h-3" />
		</Button>
	</div>
{/snippet}

{#snippet containerActions(container: ContainerInfo)}
	<div class="flex gap-1 shrink-0">
		{#if container.state !== 'running'}
			<Button
				variant="ghost"
				size="icon"
				disabled={loading[container.id]}
				onclick={() => action(container.id, 'start')}
			>
				<Play class="w-3 h-3" />
			</Button>
		{:else}
			<Button
				variant="ghost"
				size="icon"
				disabled={loading[container.id]}
				onclick={() => action(container.id, 'stop')}
			>
				<Square class="w-3 h-3" />
			</Button>
		{/if}
		<Button
			variant="ghost"
			size="icon"
			disabled={loading[container.id]}
			onclick={() => action(container.id, 'restart')}
		>
			<RotateCw class="w-3 h-3" />
		</Button>
	</div>
{/snippet}

{#snippet containerStats(container: ContainerInfo)}
	{#if container.state === 'running'}
		<div class="flex gap-3 text-muted-foreground">
			<span>CPU {container.cpu_percent.toFixed(1)}%</span>
			<span>{t('ram', locale)} {formatBytes(container.memory_usage, locale)} ({container.memory_percent.toFixed(0)}%)</span>
			{#if container.health}
				<span class="capitalize">{container.health}</span>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet containerRow(container: ContainerInfo, nested = false)}
	<div class="flex flex-col gap-1.5 p-2 rounded-md bg-secondary/50 text-xs {nested ? 'ml-3 border-l border-border/60' : ''}">
		<div class="flex items-center justify-between gap-2">
			<div class="flex items-center gap-2 min-w-0">
				<span class="w-2 h-2 rounded-full shrink-0 {stateColor(container.state)}"></span>
				<span class="font-medium truncate">{containerDisplayName(container)}</span>
				{#if container.is_arr && container.arr_kind}
					<span class="px-1.5 py-0.5 rounded bg-primary/10 text-primary text-[10px] uppercase">
						{container.arr_kind}
					</span>
				{/if}
			</div>
			{@render containerActions(container)}
		</div>
		{@render containerStats(container)}
	</div>
{/snippet}

<WidgetHeader {widget} title={widget.title ?? 'Docker'} />
<CardContent class="space-y-2">
	{#if groups.length === 0}
		<p class="text-xs text-muted-foreground">{t('docker_no_containers', locale)}</p>
	{:else}
		{#each groups as group (group.key)}
			{#if group.isCompose}
				<div class="rounded-md bg-secondary/30 text-xs overflow-hidden">
					<div class="space-y-1 p-2 hover:bg-secondary/50 transition-colors">
						<div class="flex items-center justify-between gap-2">
							<button
								type="button"
								class="flex flex-1 items-center gap-2 min-w-0 text-left"
								onclick={() => toggleGroup(group.key)}
							>
								{#if isExpanded(group)}
									<ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
								{:else}
									<ChevronRight class="size-3.5 shrink-0 text-muted-foreground" />
								{/if}
								<span
									class="w-2 h-2 rounded-full shrink-0 {stateColor(aggregateGroupState(group.containers))}"
								></span>
								<span class="font-medium truncate">{group.name}</span>
								<span class="text-muted-foreground shrink-0"
									>{runningLabel(group)} {t('docker_running', locale)}</span
								>
							</button>
							{@render groupActions(group)}
						</div>
						{#if groupHasRunning(group.containers)}
							<div class="flex gap-3 text-muted-foreground pl-5">
								<span>CPU {aggregateCpu(group.containers).toFixed(1)}%</span>
								<span
									>{t('ram', locale)}
									{formatBytes(aggregateMemory(group.containers), locale)}</span
								>
							</div>
						{/if}
					</div>
					{#if isExpanded(group)}
						<div class="space-y-1.5 px-2 pb-2 pt-2">
							{#each group.containers as container (container.id)}
								{@render containerRow(container, true)}
							{/each}
						</div>
					{/if}
				</div>
			{:else}
				{@render containerRow(group.containers[0])}
			{/if}
		{/each}
	{/if}
</CardContent>
