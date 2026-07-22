<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Trash from "$lib/icons/Trash.svelte";
	import { getDisplayIconSrc } from "$lib/icons/logos";

	let {
		instance,
		selected,
		onselect,
		onedit,
		ondelete,
	}: {
		instance: InstanceDto;
		selected: boolean;
		onselect: () => void;
		onedit: () => void;
		ondelete: () => void;
	} = $props();
</script>

<div
	class="instance-item"
	class:active={selected}
	onclick={onselect}
	onkeydown={(e) => {
		if (e.key === "Enter" || e.key === " ") onselect();
	}}
	role="button"
	tabindex="0"
	title={instance.name}
>
	<div class="instance-info-container">
		<div class="instance-icon">
			{#if instance.icon}
				<img
					src={getDisplayIconSrc(instance.icon)}
					alt={instance.name}
					width="16"
					height="16"
				/>
			{:else}
				{instance.name.charAt(0).toUpperCase()}
			{/if}
		</div>
		<span class="instance-name">{instance.name}</span>
	</div>
	<div class="instance-actions">
		<button
			type="button"
			class="action-btn"
			onclick={(e) => {
				e.stopPropagation();
				onedit();
			}}
			title={t("sidebar.rename")}
		>
			<img
				src="/images/icons/edit.svg"
				alt={t("sidebar.rename")}
				width="12"
				height="12"
				style="filter: var(--icon-filter);"
			/>
		</button>
		<button
			type="button"
			class="action-btn delete"
			onclick={(e) => {
				e.stopPropagation();
				ondelete();
			}}
			title={t("sidebar.delete")}
		>
			<Trash width="12" height="12" />
		</button>
	</div>
</div>

<style>
	.instance-item {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 1px;
		padding: 4px 10px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-primary);
		width: 100%;
		text-align: left;
	}

	.instance-item:hover {
		background: var(--surface-selected);
	}

	.instance-item.active {
		background: var(--bg-item-active);
		border-color: var(--border);
	}

	.instance-icon {
		width: 22px;
		height: 22px;
		background: rgba(var(--surface-rgb), 0.04);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		flex-shrink: 0;
	}

	.instance-info-container {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: 1;
		min-width: 0;
	}

	.instance-name {
		font-weight: 500;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.instance-actions {
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity 0.2s ease;
	}

	.instance-item:hover .instance-actions {
		opacity: 1;
	}

	@media (max-width: 650px) {
		.instance-item {
			justify-content: center;
			padding: 12px 0;
		}
	}
</style>
