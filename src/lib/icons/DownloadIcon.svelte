<script lang="ts">
	import Icon from "./Icon.svelte";

	let {
		class: className = "",
		size = 24,
	}: {
		class?: string;
		size?: number;
	} = $props();
</script>

<Icon
	name="ui:download"
	src="/images/icons/ui/download.svg"
	{size}
	class={className}
/>
