<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		variant = "grid",
		rows = 6,
		label = t("common.loading"),
	}: {
		variant?: "grid" | "tiles";
		rows?: number;
		label?: string;
	} = $props();
</script>

<div class="panel-skeleton sk-delayed" class:tiles={variant === "tiles"}>
	{#each { length: rows } as _, index (index)}
		{#if variant === "tiles"}
			<span class="sk-block panel-skeleton-tile"></span>
		{:else}
			<div class="panel-skeleton-card">
				<span
					class="sk-block"
					style="width: 44px; height: 44px; flex-shrink: 0;"
				></span>
				<div class="sk-stack panel-skeleton-text">
					<span class="sk-block sk-line" style="width: 68%"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 42%"
					></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 84%"
					></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 58%"
					></span>
				</div>
			</div>
		{/if}
	{/each}
</div>
<span class="sk-sr-only" role="status">{label}</span>

<style>
	.panel-skeleton {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(250px, 100%), 1fr));
		gap: 12px;
		align-content: start;
	}

	.panel-skeleton-card {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 14px;
		min-height: 96px;
		box-sizing: border-box;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--surface-card);
	}

	.panel-skeleton-text {
		flex: 1;
		gap: 8px;
		padding-top: 2px;
	}

	.panel-skeleton.tiles {
		grid-template-columns: repeat(auto-fill, minmax(min(180px, 100%), 1fr));
		gap: 16px;
	}

	.panel-skeleton-tile {
		aspect-ratio: 16 / 9;
	}

	@media (max-width: 550px) {
		.panel-skeleton {
			grid-template-columns: 1fr;
		}
	}
</style>
