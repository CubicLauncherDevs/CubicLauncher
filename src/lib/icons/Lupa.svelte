<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		width = "1em",
		height = "1em",
	}: {
		class?: string;
		width?: string | number;
		height?: string | number;
	} = $props();
</script>

<Icon name="ui:search" {width} {height} class={className} />
