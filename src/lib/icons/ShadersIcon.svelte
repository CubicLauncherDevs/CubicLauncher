<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		size = 18,
	}: {
		class?: string;
		size?: number;
	} = $props();
</script>

<Icon
	name="ui:shaders"
	src="/images/icons/ui/shaders.svg"
	{size}
	class={className}
/>
