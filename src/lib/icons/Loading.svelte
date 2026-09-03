<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		size = 32,
	}: {
		class?: string;
		size?: number;
	} = $props();
</script>

<Icon name="ui:spinner" {size} class={className} />
