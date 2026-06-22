<script lang="ts">
	import SunIcon from '@lucide/svelte/icons/sun';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import PaletteIcon from '@lucide/svelte/icons/palette';
	import CheckIcon from '@lucide/svelte/icons/check';
	import { userPrefersMode } from 'mode-watcher';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { THEME_PRESETS, setPreset, setThemeMode, themeState } from '$lib/theme.svelte';
	import type { ThemeMode } from '$lib/types';

	const modes: { id: ThemeMode; label: string; icon: typeof SunIcon }[] = [
		{ id: 'light', label: 'Clair', icon: SunIcon },
		{ id: 'dark', label: 'Sombre', icon: MoonIcon },
		{ id: 'system', label: 'Système', icon: MonitorIcon }
	];

	const currentMode = $derived(userPrefersMode.current);
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="outline" size="icon" class="relative">
				<SunIcon
					class="size-4 scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90"
				/>
				<MoonIcon
					class="absolute size-4 scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0"
				/>
				<span class="sr-only">Thème</span>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end" class="w-44">
		<DropdownMenu.Label>Apparence</DropdownMenu.Label>
		{#each modes as item (item.id)}
			<DropdownMenu.Item onclick={() => setThemeMode(item.id)}>
				<item.icon class="size-4" />
				{item.label}
				{#if currentMode === item.id}
					<CheckIcon class="ml-auto size-4" />
				{/if}
			</DropdownMenu.Item>
		{/each}
		<DropdownMenu.Separator />
		<DropdownMenu.Label>
			<PaletteIcon class="size-3.5 inline mr-1.5" />
			Palette
		</DropdownMenu.Label>
		{#each THEME_PRESETS as preset (preset.id)}
			<DropdownMenu.Item onclick={() => setPreset(preset.id)}>
				{preset.label}
				{#if themeState.preset === preset.id}
					<CheckIcon class="ml-auto size-4" />
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
