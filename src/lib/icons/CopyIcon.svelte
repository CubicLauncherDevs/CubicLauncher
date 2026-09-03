<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		size = 16,
	}: {
		class?: string;
		size?: number;
	} = $props();
</script>

<Icon name="ui:copy" {size} class={className} />
