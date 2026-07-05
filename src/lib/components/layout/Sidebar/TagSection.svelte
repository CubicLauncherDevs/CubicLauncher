<script lang="ts">
	import type { TagDto, InstanceDto } from "$lib/types/types";
	import InstanceItem from "./InstanceItem.svelte";

	let {
		tag,
		instances,
		selectedInstance,
		selected,
		onselect,
		onedit,
		ondelete,
		onrename,
		ondeleteTag,
	}: {
		tag: TagDto;
		instances: InstanceDto[];
		selectedInstance: InstanceDto | null;
		selected: boolean;
		onselect: (instance: InstanceDto) => void;
		onedit: (instance: InstanceDto) => void;
		ondelete: (instance: InstanceDto) => void;
		onrename: (tag: TagDto) => void;
		ondeleteTag: (tag: TagDto) => void;
	} = $props();

	let collapsed = $state(false);
</script>

<div class="tag-section">
	<button
		type="button"
		class="tag-header"
		onclick={() => (collapsed = !collapsed)}
		oncontextmenu={(e) => {
			e.stopPropagation();
		}}
	>
		<span class="tag-indicator" style="background: {tag.color ?? 'var(--text-muted)'}"></span>
		<span class="tag-name">{tag.name}</span>
		<span class="tag-count">{instances.length}</span>
		<span class="tag-chevron" class:rotated={!collapsed}>▸</span>
	</button>
	{#if !collapsed}
		<div class="tag-instances">
			{#each instances as instance (instance.uuid)}
				<InstanceItem
					{instance}
					selected={selectedInstance?.uuid === instance.uuid}
					onselect={() => onselect(instance)}
					onedit={() => onedit(instance)}
					ondelete={() => ondelete(instance)}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tag-section {
		margin-bottom: 2px;
	}

	.tag-header {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 6px 10px;
		width: 100%;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s;
		font-family: var(--font-family);
		text-align: left;
	}

	.tag-header:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.tag-indicator {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tag-name {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tag-count {
		font-size: 0.65rem;
		opacity: 0.6;
		margin-left: auto;
	}

	.tag-chevron {
		font-size: 0.6rem;
		transition: transform 0.15s;
		opacity: 0.5;
	}

	.tag-chevron.rotated {
		transform: rotate(90deg);
	}

	.tag-instances {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-left: 4px;
	}
</style>
