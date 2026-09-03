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

<Icon name="ui:shaders" {size} class={className} />
