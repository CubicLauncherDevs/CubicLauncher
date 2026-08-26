<script lang="ts">
	import type { SvelteSet } from "svelte/reactivity";
	import { LEVEL_ORDER, levelColor } from "./logHelpers";

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
		onToggleLevel: (level: string) => void;
		onSetAllLevels: (active: boolean) => void;
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
		onToggleLevel,
		onSetAllLevels,
	}: Props = $props();
</script>

<div class="log-controls">
	<div class="controls-row">
		<div class="search-group">
			<img
				class="search-icon"
				src="/images/icons/log/search.svg"
				alt=""
			/>
			<div class="input-wrap">
				<input
					type="text"
					id="log-search-input"
					class="search-input"
					value={query}
					oninput={(e) => onQueryInput(e.currentTarget.value)}
					onkeydown={onQueryKeydown}
					placeholder="Buscar en logs..."
				/>
				{#if query}
					<button
						type="button"
						class="search-clear"
						onclick={onClearQuery}
					>
						×
					</button>
				{/if}
			</div>
			<button
				type="button"
				class="nav-btn"
				onclick={onPrev}
				disabled={matchCount === 0}
				title="Anterior (Shift+Enter)"
			>
				<img
					class="nav-icon"
					src="/images/icons/log/chevron-up.svg"
					alt=""
				/>
			</button>
			<button
				type="button"
				class="nav-btn"
				onclick={onNext}
				disabled={matchCount === 0}
				title="Siguiente (Enter)"
			>
				<img
					class="nav-icon"
					src="/images/icons/log/chevron-down.svg"
					alt=""
				/>
			</button>
			<span class="match-count">
				{matchCount > 0 ? `${currentMatchIndex}/${matchCount}` : "0/0"}
			</span>
		</div>

		{#if showLevelTags}
			<div class="level-group" role="group" aria-label="Niveles de log">
				<button
					type="button"
					class="chip all"
					class:active={activeLevels.size === LEVEL_ORDER.length}
					aria-pressed={activeLevels.size === LEVEL_ORDER.length}
					onclick={() =>
						onSetAllLevels(activeLevels.size !== LEVEL_ORDER.length)}
				>
					ALL
				</button>
				{#each LEVEL_ORDER as level (level)}
					{@const isActive = activeLevels.has(level)}
					<button
						type="button"
						class="chip {level}"
						class:active={isActive}
						aria-pressed={isActive}
						style="--chip-color: {levelColor(level)}"
						onclick={() => onToggleLevel(level)}
					>
						{level.toUpperCase()}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.log-controls {
		padding: 14px 20px;
		background: var(--bg-card);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.controls-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 14px;
		flex-wrap: wrap;
	}

	.search-group {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		max-width: 520px;
	}

	.search-icon {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		filter: var(--icon-filter);
		opacity: 0.7;
	}

	.nav-icon {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		filter: var(--icon-filter);
	}

	.input-wrap {
		position: relative;
		display: flex;
		align-items: center;
		flex: 1;
		min-width: 0;
	}

	.search-input {
		width: 100%;
		background: var(--surface-input);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 8px 28px 8px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.78rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.2s ease;
	}

	.search-input:focus {
		border-color: var(--accent);
	}

	.search-clear {
		position: absolute;
		right: 8px;
		top: 50%;
		transform: translateY(-50%);
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		padding: 0 2px;
	}

	.search-clear:hover {
		color: var(--text-primary);
	}

	.nav-btn {
		width: 32px;
		height: 32px;
		border-radius: var(--border-radius-sm);
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-tertiary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
		flex-shrink: 0;
	}

	.nav-btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--text-tertiary);
		color: var(--text-primary);
	}

	.nav-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.match-count {
		color: var(--text-tertiary);
		font-size: 0.72rem;
		font-weight: 600;
		min-width: 42px;
		text-align: center;
		flex-shrink: 0;
	}

	.level-group {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}

	.chip {
		background: var(--bg-card);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 4px 9px;
		border-radius: var(--border-radius-sm);
		font-size: 0.6rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 5px;
		transition: all 0.15s ease;
	}

	.chip::before {
		content: "";
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--chip-color, currentColor);
		opacity: 0.4;
	}

	.chip.active {
		color: var(--chip-color, var(--text-primary));
		background: color-mix(
			in srgb,
			var(--chip-color, var(--surface-active)) 12%,
			var(--surface-active)
		);
		border-color: var(--chip-color, var(--border));
	}

	.chip.active::before {
		opacity: 1;
	}

	@media (max-width: 700px) {
		.controls-row {
			flex-direction: column;
			align-items: stretch;
		}

		.search-group {
			max-width: none;
		}

		.level-group {
			justify-content: flex-start;
		}
	}
</style>
