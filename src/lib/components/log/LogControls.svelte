<script lang="ts">
	import type { SvelteSet } from "svelte/reactivity";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import { LEVEL_ORDER } from "./logHelpers";

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
		<div class="level-select">
			<select
				aria-label={t("logWindow.levels")}
				class:filtered={filter !== "all"}
				value={filter}
				onchange={(event) => {
					const value = event.currentTarget.value;
					onSetLevels(
						value === "all"
							? LEVEL_ORDER
							: value === "warnings"
								? ["warn", "error", "fatal", "stderr"]
								: ["error", "fatal", "stderr"],
					);
				}}
			>
				<option value="all">{t("logWindow.all")}</option>
				<option value="warnings">{t("logWindow.warnings")}</option>
				<option value="errors">{t("logWindow.errors")}</option>
			</select>
			<span class="select-chevron" aria-hidden="true">
				<Icon name="log:chevron-down" size={12} />
			</span>
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
	button:focus-visible,
	select:focus-visible {
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
	.level-select {
		position: relative;
		flex-shrink: 0;
	}

	select {
		appearance: none;
		width: 172px;
		min-height: 34px;
		padding: 6px 30px 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease;
	}

	select:hover {
		background: var(--surface-hover);
		border-color: var(--text-tertiary);
	}

	select.filtered {
		border-color: var(--accent);
		color: var(--accent);
	}

	option {
		background: var(--bg-card);
		color: var(--text-primary);
	}

	.select-chevron {
		position: absolute;
		right: 10px;
		top: 50%;
		transform: translateY(-50%);
		display: flex;
		color: var(--text-secondary);
		pointer-events: none;
	}

	@media (max-width: 520px) {
		.log-controls {
			flex-wrap: wrap;
		}
		.search-group {
			flex-basis: 100%;
		}
		.level-select {
			margin-left: auto;
		}
	}
</style>
