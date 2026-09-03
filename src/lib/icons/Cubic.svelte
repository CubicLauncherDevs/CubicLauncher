<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		width = "100%",
		height = "100%",
	}: {
		class?: string;
		width?: string | number;
		height?: string | number;
	} = $props();
</script>

<Icon name="brand:cubic" {width} {height} class={className} />
