<script lang="ts">
	import Icon from "#lib/icons/Icon.svelte";
	import { t } from "#lib/i18n/index.js";

	let {
		loaderTab,
		LOADERS,
		onswitch,
		idPrefix,
		panelId,
	}: {
		loaderTab: string;
		LOADERS: Array<{ value: string; label: string; iconName: string }>;
		onswitch: (tab: string) => void;
		idPrefix: string;
		panelId: string;
	} = $props();

	function handleKeydown(event: KeyboardEvent, index: number) {
		let next: number;
		switch (event.key) {
			case "ArrowRight":
				next = (index + 1) % LOADERS.length;
				break;
			case "ArrowLeft":
				next = (index - 1 + LOADERS.length) % LOADERS.length;
				break;
			case "Home":
				next = 0;
				break;
			case "End":
				next = LOADERS.length - 1;
				break;
			default:
				return;
		}
		event.preventDefault();
		const target = event.currentTarget as HTMLButtonElement;
		const button =
			target.parentElement?.querySelectorAll<HTMLButtonElement>(
				"[role=tab]",
			)[next];
		button?.focus();
		button?.scrollIntoView({ block: "nearest", inline: "nearest" });
		onswitch(LOADERS[next].value);
	}
</script>

<div
	class="catalog-tabs"
	role="tablist"
	aria-label={t("versionDownloader.title")}
>
	{#each LOADERS as loader, index (loader.value)}
		<button
			type="button"
			role="tab"
			id={`${idPrefix}-${loader.value}`}
			class="catalog-tab"
			class:active={loaderTab === loader.value}
			aria-selected={loaderTab === loader.value}
			aria-controls={panelId}
			tabindex={loaderTab === loader.value ? 0 : -1}
			onclick={() => onswitch(loader.value)}
			onkeydown={(e) => handleKeydown(e, index)}
		>
			<Icon name={loader.iconName} size={18} />
			<span>{loader.label}</span>
		</button>
	{/each}
</div>

<style>
	.catalog-tabs {
		display: flex;
		flex-shrink: 0;
		gap: 4px;
		overflow-x: auto;
		border-bottom: 1px solid var(--border);
		scrollbar-width: thin;
	}
	.catalog-tab {
		position: relative;
		display: flex;
		flex: 1 0 auto;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 14px 12px;
		border: 0;
		border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		white-space: nowrap;
		cursor: pointer;
		transition:
			color 0.15s,
			background-color 0.15s;
	}
	.catalog-tab::after {
		content: "";
		position: absolute;
		bottom: 0;
		left: 12px;
		right: 12px;
		height: 3px;
		border-radius: 3px 3px 0 0;
		background: transparent;
	}
	.catalog-tab:hover {
		color: var(--text-primary);
		background: var(--surface-hover);
	}
	.catalog-tab.active {
		color: var(--text-primary);
		background: rgba(var(--accent-rgb), 0.06);
	}
	.catalog-tab.active::after {
		background: var(--accent);
	}
	.catalog-tab:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -3px;
	}
	@media (max-width: 600px) {
		.catalog-tab {
			padding: 12px 10px;
		}
	}
</style>
