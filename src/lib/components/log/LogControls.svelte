<script lang="ts">
	import type { SvelteSet } from "svelte/reactivity";
	import Icon from "$lib/icons/Icon.svelte";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import { t } from "$lib/i18n";
	import { LEVEL_ORDER } from "./logHelpers";
	import LogMenu from "./LogMenu.svelte";

	interface Props {
		activeLevels: SvelteSet<string>;
		query: string;
		matchCount: number;
		currentMatchIndex: number;
		showLevelTags?: boolean;
		onQueryInput: (value: string) => void;
		onQueryKeydown: (e: KeyboardEvent) => void;
		onClearQuery: () => void;
		onPrev: () => void;
		onNext: () => void;
		onSetLevels: (levels: string[]) => void;
	}

	let {
		activeLevels,
		query,
		matchCount,
		currentMatchIndex,
		showLevelTags = true,
		onQueryInput,
		onQueryKeydown,
		onClearQuery,
		onPrev,
		onNext,
		onSetLevels,
	}: Props = $props();
	const filter = $derived(
		activeLevels.size === LEVEL_ORDER.length
			? "all"
			: activeLevels.has("warn")
				? "warnings"
				: "errors",
	);

	const LEVEL_FILTERS: {
		value: "all" | "warnings" | "errors";
		labelKey: string;
		levels: string[];
	}[] = [
		{ value: "all", labelKey: "logWindow.all", levels: LEVEL_ORDER },
		{
			value: "warnings",
			labelKey: "logWindow.warnings",
			levels: ["warn", "error", "fatal", "stderr"],
		},
		{
			value: "errors",
			labelKey: "logWindow.errors",
			levels: ["error", "fatal", "stderr"],
		},
	];

	const currentLevelFilter = $derived(
		LEVEL_FILTERS.find((option) => option.value === filter) ??
			LEVEL_FILTERS[0],
	);
</script>

<div class="log-controls">
	<div class="search-group">
		<Icon name="log:search" size={14} />
		<input
			type="text"
			id="log-search-input"
			value={query}
			oninput={(e) => onQueryInput(e.currentTarget.value)}
			onkeydown={onQueryKeydown}
			placeholder={t("logWindow.search")}
			aria-label={t("logWindow.search")}
		/>
		{#if query}
			<span class="match-count">{currentMatchIndex}/{matchCount}</span>
			<button
				type="button"
				class="icon-button"
				onclick={onPrev}
				disabled={matchCount === 0}
				title={t("logWindow.previousMatch")}
				aria-label={t("logWindow.previousMatch")}
			>
				<Icon name="log:chevron-up" size={14} />
			</button>
			<button
				type="button"
				class="icon-button"
				onclick={onNext}
				disabled={matchCount === 0}
				title={t("logWindow.nextMatch")}
				aria-label={t("logWindow.nextMatch")}
			>
				<Icon name="log:chevron-down" size={14} />
			</button>
			<button
				type="button"
				class="icon-button"
				onclick={onClearQuery}
				title={t("logWindow.clearSearch")}
				aria-label={t("logWindow.clearSearch")}>&times;</button
			>
		{/if}
	</div>
	{#if showLevelTags}
		<div class="level-menu">
			<LogMenu
				label={t(currentLevelFilter.labelKey)}
				ariaLabel={t("logWindow.levels")}
				filtered={filter !== "all"}
			>
				{#snippet children({ close })}
					{#each LEVEL_FILTERS as option (option.value)}
						<button
							type="button"
							class="level-option"
							class:selected={filter === option.value}
							onclick={() => {
								onSetLevels([...option.levels]);
								close();
							}}
						>
							<span>{t(option.labelKey)}</span>
							{#if filter === option.value}
								<CheckIcon size={14} />
							{/if}
						</button>
					{/each}
				{/snippet}
			</LogMenu>
		</div>
	{/if}
</div>

<style>
	.log-controls,
	.search-group {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.search-group {
		flex: 1;
		min-width: 0;
		padding: 0 6px 0 10px;
		gap: 2px;
		color: var(--text-tertiary);
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}

	.search-group:focus-within {
		border-color: var(--accent);
	}

	input[type="text"] {
		flex: 1;
		min-width: 0;
		width: 100%;
		padding: 8px;
		background: transparent;
		border: none;
		outline: none;
		color: var(--text-primary);
		font: inherit;
	}

	input::placeholder {
		color: var(--text-tertiary);
	}

	button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		min-height: 32px;
		padding: 4px 8px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
		flex-shrink: 0;
	}

	button:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.icon-button {
		width: 28px;
		padding: 0;
	}
	.match-count {
		font-size: 0.65rem;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.match-count {
		margin: 0 4px;
	}
	.level-menu {
		flex-shrink: 0;
	}

	.level-option {
		justify-content: space-between;
		width: 100%;
		text-align: left;
	}

	.level-option.selected {
		background: var(--surface-selected);
		color: var(--accent);
	}

	@media (max-width: 520px) {
		.log-controls {
			flex-wrap: wrap;
		}
		.search-group {
			flex-basis: 100%;
		}
		.level-menu {
			margin-left: auto;
		}
	}
</style>
