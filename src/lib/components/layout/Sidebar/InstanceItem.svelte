<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Trash from "$lib/icons/Trash.svelte";
	import { getDisplayIconSrc, getLoaderColorVar } from "$lib/icons/logos";

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

	let isRunning = $derived(instance.status === "started");
	let loaderColor = $derived(getLoaderColorVar(instance.loader));
	let formattedLoader = $derived(
		instance.loader.charAt(0).toUpperCase() + instance.loader.slice(1),
	);
	let subtitle = $derived(
		[instance.version, formattedLoader].filter(Boolean).join(" • "),
	);
</script>

<div
	class="instance-item"
	class:active={selected}
	data-instance-uuid={instance.uuid}
	onclick={onselect}
	onkeydown={(e) => {
		if (e.key === "Enter" || e.key === " ") onselect();
	}}
	role="button"
	tabindex="0"
	title={instance.name}
>
	<div class="instance-info-container">
		<div
			class="instance-icon"
			class:has-icon={!!instance.icon}
			style={instance.icon
				? ""
				: `background: color-mix(in srgb, ${loaderColor} 18%, transparent); border-color: color-mix(in srgb, ${loaderColor} 35%, transparent); color: ${loaderColor};`}
		>
			{#if instance.icon}
				<img
					src={getDisplayIconSrc(instance.icon)}
					alt={instance.name}
					width="20"
					height="20"
				/>
			{:else}
				{instance.name.charAt(0).toUpperCase()}
				{#if isRunning}
					<span
						class="status-dot"
						aria-label={t("instanceView.status.started")}
					></span>
				{/if}
			{/if}
			{#if instance.icon && isRunning}
				<span
					class="status-dot"
					aria-label={t("instanceView.status.started")}
				></span>
			{/if}
		</div>
		<div class="instance-text">
			<span class="instance-name">{instance.name}</span>
			<span class="instance-subtitle">{subtitle}</span>
		</div>
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
				src="/images/icons/nav/edit.svg"
				alt={t("sidebar.rename")}
				width="14"
				height="14"
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
			<Trash width="14" height="14" />
		</button>
	</div>
</div>

<style>
	.instance-item {
		--item-bg: transparent;

		position: relative;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 12px;
		border-radius: var(--border-radius);
		cursor: pointer;
		transition:
			background 0.18s ease,
			border-color 0.18s ease,
			box-shadow 0.18s ease;
		border: 1px solid transparent;
		background: var(--item-bg);
		color: var(--text-primary);
		width: 100%;
		text-align: left;
		outline: none;
	}

	.instance-item:hover {
		--item-bg: var(--surface-hover);
	}

	.instance-item:focus-visible {
		--item-bg: var(--surface-hover);
		border-color: var(--text-secondary);
	}

	.instance-item.active {
		--item-bg: var(--bg-item-active);
		border-color: var(--border);
		box-shadow: inset 0 0 0 1px rgba(var(--surface-rgb), 0.04);
	}

	.instance-item.active::before {
		content: "";
		position: absolute;
		left: 0;
		top: 50%;
		transform: translateY(-50%);
		width: 3px;
		height: 18px;
		border-radius: 0 3px 3px 0;
		background: var(--accent);
	}

	.instance-info-container {
		display: flex;
		align-items: center;
		gap: 12px;
		flex: 1;
		min-width: 0;
	}

	.instance-icon {
		position: relative;
		width: 28px;
		height: 28px;
		background: rgba(var(--surface-rgb), 0.04);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.85rem;
		font-weight: 700;
		flex-shrink: 0;
		transition:
			transform 0.18s ease,
			border-color 0.18s ease;
	}

	.instance-item:hover .instance-icon:not(.has-icon) {
		transform: scale(1.05);
	}

	.instance-icon img {
		display: block;
		border-radius: 2px;
		object-fit: cover;
	}

	.instance-text {
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
		flex: 1;
	}

	.instance-name {
		font-weight: 600;
		font-size: 0.85rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		transition: color 0.15s ease;
	}

	.instance-item.active .instance-name {
		color: var(--text-primary);
	}

	.instance-subtitle {
		font-size: 0.7rem;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		transition: color 0.15s ease;
	}

	.instance-item:hover .instance-subtitle,
	.instance-item.active .instance-subtitle {
		color: var(--text-secondary);
	}

	.instance-actions {
		display: flex;
		align-items: center;
		gap: 2px;
		opacity: 0;
		transition: opacity 0.2s ease;
	}

	.instance-item:hover .instance-actions,
	.instance-item.active .instance-actions,
	.instance-item:focus-within .instance-actions {
		opacity: 1;
	}

	.action-btn {
		width: 26px;
		height: 26px;
		border-radius: var(--border-radius-sm);
		border: none;
		background: transparent;
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			transform 0.12s ease;
	}

	.action-btn:hover {
		background: rgba(var(--surface-rgb), 0.08);
		color: var(--text-primary);
	}

	.action-btn:active {
		transform: scale(0.94);
	}

	.action-btn.delete:hover {
		background: rgba(var(--color-error-rgb), 0.12);
		color: var(--color-error);
	}

	.status-dot {
		position: absolute;
		bottom: -2px;
		right: -2px;
		width: 9px;
		height: 9px;
		border-radius: 50%;
		background: var(--color-status-started);
		box-shadow: 0 0 0 2px var(--item-bg);
		transition: box-shadow 0.18s ease;
	}

	@media (max-width: 650px) {
		.instance-item {
			justify-content: center;
			padding: 12px 0;
		}

		.instance-text,
		.instance-actions {
			display: none;
		}
	}
</style>
