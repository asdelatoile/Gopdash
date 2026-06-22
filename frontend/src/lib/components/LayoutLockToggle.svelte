<script lang="ts">
	import LockIcon from '@lucide/svelte/icons/lock';
	import LockOpenIcon from '@lucide/svelte/icons/lock-open';
	import { Button } from '$lib/components/ui/button/index.js';
	import { layoutState, toggleLayoutLock } from '$lib/layout.svelte';
</script>

<Button
	variant={layoutState.locked ? 'outline' : 'secondary'}
	size="icon"
	onclick={() => toggleLayoutLock()}
	title={layoutState.locked ? 'Déverrouiller la disposition' : 'Verrouiller la disposition'}
	aria-label={layoutState.locked ? 'Déverrouiller la disposition' : 'Verrouiller la disposition'}
	aria-pressed={!layoutState.locked}
>
	{#if layoutState.locked}
		<LockIcon class="size-4" />
	{:else}
		<LockOpenIcon class="size-4" />
	{/if}
</Button>
