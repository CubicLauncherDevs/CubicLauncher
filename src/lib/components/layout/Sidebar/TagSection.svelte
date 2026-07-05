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
		class:expanded={!collapsed}
		style="border-left-color: {tag.color ?? 'var(--text-muted)'}"
		onclick={() => (collapsed = !collapsed)}
		oncontextmenu={(e) => {
			e.stopPropagation();
		}}
	>
		<span class="tag-name">{tag.name}</span>
		<span class="tag-count">{instances.length}</span>
		<span class="tag-chevron" class:rotated={!collapsed}>▸</span>
	</button>
	<div class="tag-body" class:expanded={!collapsed}>
		<div class="instances-inner">
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
	</div>
</div>

<style>
	.tag-header {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 6px 10px;
		width: 100%;
		background: transparent;
		border: none;
		border-left: 3px solid var(--text-muted);
		border-radius: 0 var(--border-radius-sm) var(--border-radius-sm) 0;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		font-family: var(--font-family);
		text-align: left;
		transition: background 0.15s;
	}

	.tag-header:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.tag-name {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tag-count {
		font-size: 0.65rem;
		opacity: 0.5;
		margin-left: auto;
	}

	.tag-chevron {
		font-size: 0.6rem;
		transition: transform 0.2s;
		opacity: 0.4;
		line-height: 1;
	}

	.tag-chevron.rotated {
		transform: rotate(90deg);
	}

	.tag-body {
		display: grid;
		grid-template-rows: 0fr;
		transition: grid-template-rows 0.2s ease;
	}

	.tag-body.expanded {
		grid-template-rows: 1fr;
	}

	.instances-inner {
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow: hidden;
		border-left: 2px solid var(--border-color);
		margin-left: 15px;
		padding-left: 8px;
	}
</style>
