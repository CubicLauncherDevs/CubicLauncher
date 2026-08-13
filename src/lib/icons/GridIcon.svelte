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

<Icon
	name="instance:grid"
	src="/images/icons/instance/grid.svg"
	{size}
	class={className}
/>
