<script lang="ts">
	interface Props {
		class?: string;
		alt?: string;
	}

	let { class: className = 'size-8', alt = 'GopDash' }: Props = $props();
</script>

<img src="/app-icon.svg" {alt} class={className} decoding="async" />
