<script lang="ts">
	import { t } from "$lib/i18n";
	import type { MarketProject } from "$lib/types/market";
	import Loading from "$lib/icons/Loading.svelte";

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

	const remoteLabel = $derived.by(() => {
		if (project.hasRemoteData) {
			return project.modrinthProjectId ? "Modrinth" : "CF";
		}
		return null;
	});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="market-item"
	class:selected
	class:disabled={project.disabled}
	class:incompatible
	onclick={onSelect}
>
	<div class="market-item-icon">
		{#if project.icon}
			<img src={project.icon} alt={project.title} loading="lazy" />
		{:else}
			<span class="market-item-icon-fallback">📦</span>
		{/if}
	</div>

	<div class="market-item-body">
		<div class="market-item-header">
			<h4 class="market-item-title" title={project.title}>
				{project.title}
			</h4>
			<div class="market-item-badges">
				{#if statusLabel}
					<span class="market-item-badge update">
						{statusLabel}
					</span>
				{/if}
				{#if remoteLabel}
					<span class="market-item-badge remote">
						{remoteLabel}
					</span>
				{/if}
				{#if incompatible}
					<span class="market-item-badge incompatible">
						{t("market.item.incompatible")}
					</span>
				{/if}
			</div>
		</div>

		<span class="market-item-author">
			{t("market.item.by")}
			{project.author || t("market.item.unknownAuthor")}
		</span>

		<p class="market-item-description" title={project.description}>
			{project.description || t("market.item.noDescription")}
		</p>
	</div>

	<div class="market-item-actions">
		{#if project.downloadCount > 0}
			<span class="market-item-downloads">
				↓ {formatNumber(project.downloadCount)}
			</span>
		{/if}

		{#if project.installed}
			<span class="market-item-installed-badge"
				>{t("market.item.installed")}</span
			>
		{:else if onInstall}
			<button
				type="button"
				class="market-item-install-btn"
				disabled={installing}
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
		align-items: stretch;
		gap: 14px;
		padding: 12px 14px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
		min-height: 90px;
		box-sizing: border-box;
	}

	.market-item:hover {
		background: rgba(255, 255, 255, 0.04);
		border-color: rgba(255, 255, 255, 0.15);
	}

	.market-item.selected {
		border-color: var(--accent);
		background: rgba(255, 255, 255, 0.04);
	}

	.market-item.disabled {
		opacity: 0.5;
		filter: grayscale(1);
	}

	.market-item.incompatible {
		border-left: 3px solid var(--color-error);
	}

	.market-item-icon {
		width: 56px;
		height: 56px;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		overflow: hidden;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		align-self: center;
	}

	.market-item-icon img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		image-rendering: pixelated;
	}

	.market-item-icon-fallback {
		font-size: 1.3rem;
		opacity: 0.6;
	}

	.market-item-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 3px;
		overflow: hidden;
	}

	.market-item-header {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.market-item-title {
		font-size: 0.86rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		flex: 1;
		min-width: 0;
	}

	.market-item-badges {
		display: flex;
		align-items: center;
		gap: 4px;
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
		background: rgba(var(--accent-rgb, 255 255 255) / 0.1);
		border: 1px solid rgba(var(--accent-rgb, 255 255 255) / 0.3);
	}

	.market-item-badge.incompatible {
		color: var(--color-error);
		background: rgba(var(--color-error-rgb), 0.08);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
	}

	.market-item-badge.remote {
		color: var(--text-primary);
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid var(--border);
		font-size: 0.55rem;
	}

	.market-item-author {
		font-size: 0.7rem;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.market-item-description {
		font-size: 0.75rem;
		color: var(--text-secondary);
		line-height: 1.35;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.market-item-actions {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		justify-content: center;
		gap: 6px;
		flex-shrink: 0;
	}

	.market-item-downloads {
		font-size: 0.7rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid var(--border);
		padding: 2px 7px;
		border-radius: var(--border-radius-sm);
		white-space: nowrap;
	}

	.market-item-install-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 5px 12px;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition: filter 0.15s;
		white-space: nowrap;
	}

	.market-item-install-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}

	.market-item-install-btn:disabled {
		opacity: 0.6;
		cursor: wait;
	}

	.market-item-installed-badge {
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		padding: 3px 8px;
		border-radius: var(--border-radius-sm);
		color: var(--color-success);
		background: rgba(var(--color-success-rgb), 0.1);
		border: 1px solid rgba(var(--color-success-rgb), 0.25);
		white-space: nowrap;
	}

	:global(.item-install-spinner) {
		width: 14px;
		height: 14px;
	}
</style>
