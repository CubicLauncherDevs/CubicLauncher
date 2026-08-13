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

<Icon
	name="ui:spinner"
	src="/images/icons/ui/spinner.svg"
	{size}
	class={className}
/>
