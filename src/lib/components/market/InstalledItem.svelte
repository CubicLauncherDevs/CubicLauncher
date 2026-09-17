<script lang="ts">
	import { t } from "$lib/i18n";
	import type { MarketProject } from "$lib/types/market";
	import CubicLogo from "./CubicLogo.svelte";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		project,
		icon = project.icon,
		checked,
		compact,
		busy,
		canToggle,
		onCheck,
		onOpen,
		onToggle,
		onDelete,
	}: {
		project: MarketProject;
		icon?: string | null;
		checked: boolean;
		compact: boolean;
		busy: boolean;
		canToggle: boolean;
		onCheck: () => void;
		onOpen: () => void;
		onToggle: () => void;
		onDelete: () => void;
	} = $props();
	let iconError = $state(false);
	$effect(() => {
		void icon;
		iconError = false;
	});
	const source = $derived(
		project.source === "modrinth"
			? "Modrinth"
			: project.source === "curseforge"
				? "CurseForge"
				: t("market.item.local"),
	);
</script>

<div class="installed-item" class:compact class:checked>
	<input
		type="checkbox"
		{checked}
		disabled={busy}
		onchange={onCheck}
		aria-label={t("market.manage.selectItem", { name: project.title })}
	/>
	<button
		type="button"
		class="open-item"
		onclick={onOpen}
		title={project.installed?.filename}
	>
		<span class="item-icon">
			{#if icon && !iconError}
				<img
					src={icon}
					alt=""
					loading="lazy"
					decoding="async"
					onerror={() => (iconError = true)}
				/>
			{:else}
				<CubicLogo />
			{/if}
		</span>
		<span class="item-info">
			<strong>{project.title}</strong>
			<span class="filename">{project.installed?.filename}</span>
			<span class="metadata">
				<span
					>{project.installed?.version ||
						t("market.manage.unknownVersion")}</span
				>
				<span>{source}</span>
				<span
					class="status-badge"
					class:muted={canToggle && project.disabled}
				>
					{canToggle
						? t(
								project.disabled
									? "market.manage.disabled"
									: "market.manage.enabled",
							)
						: t("market.item.installed")}
				</span>
			</span>
		</span>
	</button>
	<div class="item-actions">
		{#if canToggle}
			<button
				type="button"
				class="toggle"
				disabled={busy}
				onclick={onToggle}
				aria-label={`${t(project.disabled ? "market.detail.enable" : "market.detail.disable")} ${project.title}`}
			>
				<Icon
					name={project.disabled ? "ui:check" : "ui:close"}
					size={14}
				/>
				{t(
					project.disabled
						? "market.detail.enable"
						: "market.detail.disable",
				)}
			</button>
		{/if}
		<button
			type="button"
			class="delete"
			disabled={busy}
			onclick={onDelete}
			title={t("market.detail.uninstall")}
			aria-label={`${t("market.detail.uninstall")} ${project.title}`}
		>
			<Icon name="ui:trash" size={16} />
		</button>
	</div>
</div>

<style>
	.installed-item {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr) auto;
		gap: 10px;
		height: 100%;
		box-sizing: border-box;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
		color: var(--text-primary);
		min-width: 0;
		transition:
			border-color 120ms ease,
			background-color 120ms ease;
	}
	.installed-item:hover,
	.installed-item:focus-within {
		background: var(--surface-hover);
	}
	.installed-item.checked {
		border-color: var(--accent);
		background: var(--surface-selected);
	}
	.installed-item.compact {
		grid-template-columns: auto minmax(0, 1fr) auto;
		grid-template-rows: 1fr;
		align-items: center;
		height: calc(100% - 6px);
		padding: 8px 12px;
	}
	input {
		appearance: none;
		-webkit-appearance: none;
		width: 18px;
		height: 18px;
		box-sizing: border-box;
		margin: 4px 0;
		background: var(--bg-input);
		border: 1px solid var(--border-hover);
		border-radius: var(--border-radius-sm);
		position: relative;
		cursor: pointer;
		align-self: start;
	}
	.compact input {
		align-self: center;
	}
	input:hover:not(:disabled) {
		border-color: var(--accent);
	}
	input:checked {
		background: var(--accent);
		border-color: var(--accent);
	}
	input:checked::after {
		content: "";
		position: absolute;
		width: 5px;
		height: 9px;
		border: solid var(--accent-text);
		border-width: 0 2px 2px 0;
		top: 1px;
		left: 5px;
		transform: rotate(45deg);
	}
	button {
		font: inherit;
		color: inherit;
		cursor: pointer;
	}
	button:focus-visible,
	input:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	button:disabled,
	input:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.open-item {
		display: flex;
		align-items: center;
		gap: 12px;
		border: 0;
		background: transparent;
		text-align: left;
		padding: 0;
		min-width: 0;
	}
	.item-icon {
		width: 40px;
		height: 40px;
		flex-shrink: 0;
		display: grid;
		place-items: center;
		padding: 4px;
		box-sizing: border-box;
		background: var(--surface-subtle);
		border-radius: var(--border-radius-sm);
	}
	.item-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		border-radius: var(--border-radius-sm);
	}
	.item-info {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
	}
	strong,
	.filename {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	strong {
		font-size: 0.9rem;
	}
	.filename {
		font-size: 0.75rem;
		color: var(--text-secondary);
	}
	.metadata {
		display: flex;
		gap: 8px;
		font-size: 0.7rem;
		align-items: center;
		color: var(--text-secondary);
		overflow: hidden;
		white-space: nowrap;
	}
	.metadata > span {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.status-badge {
		padding: 1px 5px;
		border: 1px solid var(--border-hover);
		border-radius: var(--border-radius-sm);
		background: var(--surface-raised);
		color: var(--text-primary);
		line-height: 1.2;
	}
	.status-badge.muted {
		color: var(--text-muted);
		border-color: var(--border);
		background: transparent;
	}
	.item-actions {
		grid-column: 1 / -1;
		display: flex;
		gap: 8px;
		justify-content: flex-end;
		align-items: end;
		border-top: 1px solid var(--border);
		padding-top: 8px;
	}
	.compact .item-actions {
		grid-column: auto;
		align-items: center;
		border-top: 0;
		padding-top: 0;
	}
	.item-actions button {
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		min-height: 32px;
		padding: 6px 10px;
		font-size: 0.75rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		line-height: 1.2;
		transition:
			background-color 120ms ease,
			border-color 120ms ease;
	}
	.item-actions button:hover:not(:disabled) {
		border-color: var(--accent);
		background: var(--surface-hover);
	}
	.item-actions .delete {
		display: grid;
		place-items: center;
		color: var(--color-error);
	}
	.item-actions .delete:hover:not(:disabled) {
		border-color: var(--color-error);
		background: color-mix(
			in srgb,
			var(--color-error) 10%,
			var(--surface-card)
		);
	}
	@media (prefers-reduced-motion: reduce) {
		.installed-item,
		.item-actions button {
			transition: none;
		}
	}
	@container market (max-width: 550px) {
		.compact .item-icon {
			display: none;
		}
		.compact {
			gap: 6px;
			padding: 8px;
		}
		.compact .item-actions {
			gap: 4px;
		}
		.compact .item-actions button {
			padding: 6px;
		}
	}
</style>
