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
	name="instance:image"
	src="/images/icons/instance/image.svg"
	{size}
	class={className}
/>
