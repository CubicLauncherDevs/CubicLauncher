<script lang="ts">
	import Icon from "./Icon.svelte";
	let { size = 18 }: { size?: number } = $props();
</script>

<Icon name="instance:worlds" {size} />
