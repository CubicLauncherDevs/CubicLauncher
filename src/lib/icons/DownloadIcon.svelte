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

<Icon name="ui:download" {size} class={className} />
