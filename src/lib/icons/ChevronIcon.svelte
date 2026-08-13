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
	name="ui:chevron-down"
	src="/images/icons/ui/chevron-down.svg"
	{size}
	class={className}
/>
