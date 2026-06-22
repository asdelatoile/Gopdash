<script lang="ts">
	import CircleHelpIcon from '@lucide/svelte/icons/circle-help';
	import { cn } from '$lib/utils';
	import { lucideIconUrl, parseIcon } from '$lib/icons';

	interface Props {
		value?: string | null;
		class?: string;
		alt?: string;
	}

	let { value = null, class: className = '', alt = '' }: Props = $props();

	const parsed = $derived(parseIcon(value));
	let lucideFailed = $state(false);

	$effect(() => {
		if (parsed?.kind !== 'lucide') {
			lucideFailed = false;
			return;
		}

		lucideFailed = false;
		const url = lucideIconUrl(parsed.name);
		const img = new Image();
		img.onload = () => {
			lucideFailed = false;
		};
		img.onerror = () => {
			lucideFailed = true;
		};
		img.src = url;
	});
</script>

{#if parsed?.kind === 'image'}
	<img src={parsed.src} {alt} class={cn('shrink-0 object-contain', className)} />
{:else if parsed?.kind === 'lucide' && !lucideFailed}
	<span
		role="img"
		aria-label={alt || parsed.name}
		class={cn('inline-block shrink-0 bg-current', className)}
		style:mask-image="url({lucideIconUrl(parsed.name)})"
		style:-webkit-mask-image="url({lucideIconUrl(parsed.name)})"
		style:mask-size="contain"
		style:-webkit-mask-size="contain"
		style:mask-repeat="no-repeat"
		style:-webkit-mask-repeat="no-repeat"
		style:mask-position="center"
		style:-webkit-mask-position="center"
	></span>
{:else if parsed?.kind === 'lucide'}
	<CircleHelpIcon class={cn('shrink-0 text-muted-foreground', className)} />
{/if}
