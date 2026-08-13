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

<Icon
	name="brand:cubic"
	src="/images/cubic.svg"
	{width}
	{height}
	class={className}
/>
