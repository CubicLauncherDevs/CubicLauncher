<script lang="ts">
	import type { SvelteSet } from "svelte/reactivity";
	import { LEVEL_ORDER, levelColor } from "./logHelpers";

	interface Props {
		activeLevels: SvelteSet<string>;
		query: string;
		matchCount: number;
		currentMatchIndex: number;
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
	<div class="search-box">
		<svg
			class="search-icon"
			width="12"
			height="12"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<circle cx="11" cy="11" r="8" /><line
				x1="21"
				y1="21"
				x2="16.65"
				y2="16.65"
			/>
		</svg>
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
			<button type="button" class="search-clear" onclick={onClearQuery}>
				×
			</button>
		{/if}
		<button
			type="button"
			class="toolbar-btn"
			onclick={onPrev}
			disabled={matchCount === 0}
			title="Anterior (Shift+Enter)"
		>
			↑
		</button>
		<button
			type="button"
			class="toolbar-btn"
			onclick={onNext}
			disabled={matchCount === 0}
			title="Siguiente (Enter)"
		>
			↓
		</button>
		<span class="match-count">
			{matchCount > 0 ? `${currentMatchIndex}/${matchCount}` : "0/0"}
		</span>
	</div>

	<div class="level-chips">
		<button
			type="button"
			class="chip all"
			class:active={activeLevels.size === LEVEL_ORDER.length}
			onclick={() =>
				onSetAllLevels(activeLevels.size !== LEVEL_ORDER.length)}
		>
			ALL
		</button>
		{#each LEVEL_ORDER as level (level)}
			<button
				type="button"
				class="chip {level}"
				class:active={activeLevels.has(level)}
				style="--chip-color: {levelColor(level)}"
				onclick={() => onToggleLevel(level)}
			>
				{level.toUpperCase()}
			</button>
		{/each}
	</div>
</div>

<style>
	.log-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 6px 14px;
		background: var(--bg-card, #111);
		border-bottom: 1px solid var(--border, #222);
		flex-shrink: 0;
		flex-wrap: wrap;
	}

	.search-box {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.search-icon {
		color: var(--text-muted, #666);
		flex-shrink: 0;
	}

	.search-input {
		background: var(--bg-input, #0a0a0a);
		border: 1px solid var(--border, #333);
		color: var(--text-primary, #ccc);
		padding: 3px 8px;
		border-radius: 4px;
		font-size: 0.6rem;
		font-family: inherit;
		width: 160px;
		outline: none;
	}

	.search-input:focus {
		border-color: var(--text-secondary, #666);
	}

	.search-clear {
		background: transparent;
		border: none;
		color: var(--text-secondary, #999);
		cursor: pointer;
		font-size: 0.9rem;
		line-height: 1;
		padding: 0 2px;
	}

	.search-clear:hover {
		color: var(--text-primary, #fff);
	}

	.match-count {
		color: var(--text-tertiary, #888);
		font-size: 0.55rem;
		min-width: 34px;
		text-align: center;
	}

	.level-chips {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-wrap: wrap;
	}

	.chip {
		background: transparent;
		border: 1px solid var(--border, #333);
		color: var(--chip-color, var(--text-secondary, #888));
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 0.52rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 4px;
		transition: all 0.12s ease;
	}

	.chip::before {
		content: "";
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: currentColor;
		opacity: 0.5;
	}

	.chip.active {
		background: color-mix(
			in srgb,
			var(--chip-color, var(--text-secondary, #888)) 15%,
			transparent
		);
		border-color: var(--chip-color, var(--text-secondary, #888));
	}

	.chip.active::before {
		opacity: 1;
	}

	.toolbar-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary, #666);
		width: 26px;
		height: 26px;
		border-radius: 4px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
		font-size: 0.75rem;
	}

	.toolbar-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary, #ccc);
	}

	.toolbar-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
</style>
