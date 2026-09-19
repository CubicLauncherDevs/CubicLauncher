<script lang="ts">
	import { t } from "$lib/i18n";
	import type { MarketProject } from "$lib/types/market";
	import Loading from "$lib/icons/Loading.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import CubicLogo from "./CubicLogo.svelte";

	interface Props {
		project: MarketProject;
		selected?: boolean;
		incompatible?: boolean;
		onSelect: () => void;
		onInstall?: () => void;
	}

	let {
		project,
		selected = false,
		incompatible = false,
		onSelect,
		onInstall,
	}: Props = $props();

	let installing = $state(false);
	let iconError = $state(false);

	function formatNumber(num: number): string {
		if (num >= 1_000_000) return (num / 1_000_000).toFixed(1) + "M";
		if (num >= 1_000) return (num / 1_000).toFixed(1) + "K";
		return num.toString();
	}

	async function handleInstall(e: Event) {
		e.stopPropagation();
		if (!onInstall || installing) return;
		installing = true;
		try {
			await onInstall();
		} finally {
			installing = false;
		}
	}

	const statusLabel = $derived.by(() => {
		if (project.hasUpdate) return t("market.item.updateAvailable");
		return null;
	});

	const sourceLabel = $derived.by(() => {
		switch (project.source) {
			case "modrinth":
				return "Modrinth";
			case "curseforge":
				return "CurseForge";
			case "local":
				return t("market.item.local");
			default:
				return null;
		}
	});
</script>

<div
	class="market-item"
	class:selected
	class:disabled={project.disabled}
	class:incompatible
>
	<button
		type="button"
		class="market-item-open"
		onclick={onSelect}
		aria-label={project.title}
	>
		<span class="market-item-icon">
			{#if project.icon && !iconError}
				<img
					src={project.icon}
					alt={project.title}
					loading="lazy"
					decoding="async"
					onerror={() => (iconError = true)}
				/>
			{:else}
				<CubicLogo />
			{/if}
		</span>

		<span class="market-item-body">
			<span class="market-item-header">
				<span class="market-item-title" title={project.title}>
					{project.title}
				</span>
				{#if statusLabel || incompatible}
					<span class="market-item-badges">
						{#if statusLabel}
							<span class="market-item-badge update">
								{statusLabel}
							</span>
						{/if}
						{#if incompatible}
							<span class="market-item-badge incompatible">
								{t("market.item.incompatible")}
							</span>
						{/if}
					</span>
				{/if}
			</span>

			<span class="market-item-author">
				{t("market.item.by")}
				{project.author || t("market.item.unknownAuthor")}
			</span>
		</span>
		<span class="market-item-description" title={project.description}>
			{project.description || t("market.item.noDescription")}
		</span>
	</button>

	<div class="market-item-actions">
		<div class="market-item-meta">
			{#if sourceLabel}
				<span class="market-item-source">{sourceLabel}</span>
			{/if}
			{#if project.downloadCount > 0}
				<span
					class="market-item-downloads"
					title={`${t("market.detail.downloads")}: ${project.downloadCount.toLocaleString()}`}
				>
					<Icon name="ui:download" size={12} />{formatNumber(
						project.downloadCount,
					)}
				</span>
			{/if}
		</div>

		{#if project.installed}
			<span class="market-item-installed-badge"
				><Icon name="ui:check" size={12} />{t(
					"market.item.installed",
				)}</span
			>
		{:else if onInstall}
			<button
				type="button"
				class="market-item-install-btn"
				disabled={installing}
				aria-label={`${t("market.item.install")} ${project.title}`}
				aria-busy={installing}
				onclick={handleInstall}
			>
				{#if installing}
					<Loading class="item-install-spinner" />
				{:else}
					{t("market.item.install")}
				{/if}
			</button>
		{/if}
	</div>
</div>

<style>
	.market-item {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-md);
		padding: var(--market-card-padding);
		background: var(--bg-card-gradient), var(--surface-card);
		border: var(--border-width) solid var(--border);
		border-radius: var(--market-card-radius);
		transition:
			border-color var(--transition-fast) ease,
			background-color var(--transition-fast) ease;
		height: 100%;
		min-width: 0;
		box-sizing: border-box;
	}

	.market-item:hover,
	.market-item:focus-within {
		background: var(--surface-hover);
		border-color: var(--border-hover);
	}

	.market-item-open {
		display: grid;
		grid-template-columns: var(--market-icon-size) minmax(0, 1fr);
		grid-template-rows: auto 1fr;
		align-items: start;
		gap: var(--market-item-gap);
		flex: 1;
		min-height: 0;
		min-width: 0;
		padding: 0;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: start;
		cursor: pointer;
		border-radius: var(--border-radius-sm);
	}

	.market-item-open:focus-visible,
	.market-item-install-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 4px;
	}

	.market-item.selected {
		border-color: var(--accent);
		background: var(--bg-card-gradient), var(--surface-selected);
	}

	.market-item.disabled {
		opacity: 0.5;
		filter: grayscale(1);
	}

	.market-item.incompatible {
		border-left: 3px solid var(--color-error);
	}

	.market-item-icon {
		width: var(--market-icon-size);
		height: var(--market-icon-size);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		border: var(--border-width) solid var(--border);
		overflow: hidden;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		align-self: flex-start;
	}

	.market-item-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.market-item-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: flex-start;
		gap: 6px;
		overflow: hidden;
	}

	.market-item-header {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
		min-width: 0;
	}

	.market-item-title {
		font-size: var(--market-title-size);
		font-weight: var(--font-weight-bold);
		color: var(--text-primary);
		margin: 0;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		line-height: 1.3;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	.market-item-badges {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-wrap: wrap;
		flex-shrink: 0;
	}

	.market-item-badge {
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		padding: 2px 6px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}

	.market-item-badge.update {
		color: var(--accent);
		background: color-mix(in srgb, var(--accent) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.market-item-badge.incompatible {
		color: var(--color-error);
		background: color-mix(in srgb, var(--color-error) 8%, transparent);
		border: var(--border-width) solid
			color-mix(in srgb, var(--color-error) 25%, transparent);
	}

	.market-item-author {
		font-size: 0.7rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.market-item-description {
		grid-column: 1 / -1;
		min-height: 0;
		font-size: var(--market-description-size);
		color: var(--text-tertiary, var(--text-secondary));
		line-height: var(--line-height);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.market-item-actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 28px;
		gap: 6px;
		flex-shrink: 0;
		padding-top: 10px;
		border-top: var(--border-width) solid var(--border);
	}

	.market-item-meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 10px;
		min-width: 0;
	}

	.market-item-source {
		font-size: 0.65rem;
		color: var(--text-tertiary, var(--text-secondary));
	}

	.market-item-downloads {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 0.7rem;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.market-item-install-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: var(--market-button-padding);
		background: var(--surface-input);
		color: var(--text-primary);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition:
			background-color 0.15s,
			color 0.15s;
		white-space: nowrap;
	}

	.market-item-install-btn:hover:not(:disabled) {
		background: var(--accent);
		color: var(--accent-text);
	}

	.market-item-install-btn:disabled {
		opacity: 0.6;
		cursor: wait;
	}

	.market-item-installed-badge {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		padding: 3px 8px;
		border-radius: var(--border-radius-sm);
		color: var(--color-success);
		background: color-mix(in srgb, var(--color-success) 10%, transparent);
		border: var(--border-width) solid
			color-mix(in srgb, var(--color-success) 25%, transparent);
		white-space: nowrap;
	}

	:global(.item-install-spinner) {
		width: 14px;
		height: 14px;
	}
	@media (prefers-reduced-motion: reduce) {
		.market-item,
		.market-item-install-btn {
			transition: none;
		}
	}
</style>
