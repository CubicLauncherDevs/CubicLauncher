<script lang="ts">
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Trash from "$lib/icons/Trash.svelte";
	import Icon from "$lib/icons/Icon.svelte";
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

	let isRunning = $derived(instance.status === "started");
	let isPinned = $derived(instance.pinned);
	let iconUrl = $derived(getDisplayIconSrc(instance.icon));
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
	<div class="instance-icon-strip">
		<div class="instance-icon-inner">
			{#if instance.icon}
				<img
					src={iconUrl}
					alt={instance.name}
					loading="lazy"
					decoding="async"
					width="22"
					height="22"
				/>
			{:else}
				<span class="instance-icon-letter">
					{instance.name.charAt(0).toUpperCase()}
				</span>
			{/if}
			{#if isRunning}
				<span
					class="status-dot"
					aria-label={t("instanceView.status.started")}
				></span>
			{/if}
			{#if isPinned}
				<span class="pin-icon">
					<Icon src="/images/icons/ui/pin.svg" size={10} />
				</span>
			{/if}
		</div>
	</div>

	<div class="instance-text">
		<span class="instance-name">{instance.name}</span>
		<span class="instance-subtitle">{subtitle}</span>
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
		--item-bg: var(--bg-card);

		position: relative;
		display: flex;
		align-items: center;
		border-radius: var(--border-radius);
		cursor: pointer;
		transition:
			background 0.18s ease,
			border-color 0.18s ease,
			box-shadow 0.18s ease;
		border: 1px solid var(--border);
		background: var(--item-bg);
		color: var(--text-primary);
		width: 100%;
		text-align: left;
		outline: none;
		height: 38px;
		min-height: 38px;
		margin-bottom: 4px;
		box-sizing: border-box;
		overflow: hidden;
		contain: layout paint style;
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
	}

	.instance-icon-strip {
		flex-shrink: 0;
		width: 38px;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		border-right: 1px solid var(--border);
		position: relative;
		z-index: 0;
		color: var(--text-primary);
		transition: color 0.18s ease;
	}

	.instance-icon-strip::before {
		content: "";
		position: absolute;
		inset: 0;
		background: var(--surface-active);
		filter: brightness(0.8);
		transition:
			background 0.18s ease,
			filter 0.18s ease;
		z-index: 0;
	}

	.instance-item.active .instance-icon-strip {
		color: var(--accent-text);
	}

	.instance-item.active .instance-icon-strip::before {
		background: var(--accent);
		filter: brightness(0.5);
	}

	.instance-icon-inner {
		position: relative;
		width: 22px;
		height: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1;
	}

	.instance-icon-inner img {
		display: block;
		border-radius: 2px;
		object-fit: cover;
	}

	.instance-icon-letter {
		font-size: 0.85rem;
		font-weight: 700;
		line-height: 1;
	}

	.instance-text {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
		padding: 0 8px;
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
		display: none;
		align-items: center;
		gap: 2px;
		padding-right: 6px;
		opacity: 0;
		transition: opacity 0.2s ease;
		transition-behavior: allow-discrete;
	}

	@starting-style {
		.instance-item:hover .instance-actions,
		.instance-item.active .instance-actions,
		.instance-item:focus-within .instance-actions {
			opacity: 0;
		}
	}

	.instance-item:hover .instance-actions,
	.instance-item.active .instance-actions,
	.instance-item:focus-within .instance-actions {
		display: flex;
		opacity: 1;
	}

	.action-btn {
		width: 24px;
		height: 24px;
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
		bottom: -3px;
		right: -3px;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-status-started);
		box-shadow: 0 0 0 2px var(--item-bg);
		transition: box-shadow 0.18s ease;
	}

	.pin-icon {
		position: absolute;
		top: -4px;
		right: -4px;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 14px;
		height: 14px;
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: 50%;
		color: var(--color-warning);
		pointer-events: none;
		z-index: 1;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
	}

	@media (max-width: 650px) {
		.instance-item {
			justify-content: center;
		}

		.instance-text,
		.instance-actions {
			display: none;
		}
	}
</style>
