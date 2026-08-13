<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		size = 16,
		color,
	}: {
		class?: string;
		size?: number;
		color?: string | null;
	} = $props();
</script>

<Icon
	name="ui:check"
	src="/images/icons/ui/check.svg"
	{size}
	{color}
	class={className}
/>
