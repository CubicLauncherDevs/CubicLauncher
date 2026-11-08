<script lang="ts">
	import MarketSkeleton from "./MarketSkeleton.svelte";
	import { t } from "$lib/i18n";
</script>

<div class="market-catalog-skeleton sk-delayed" aria-hidden="true">
	<div class="filter-panel-skeleton">
		<div class="search-heading">
			<span class="sk-block heading-title"></span>
			<div class="compatibility">
				<span class="sk-block chip"></span>
				<span class="sk-block chip short"></span>
			</div>
		</div>

		<div class="search-row-skeleton">
			<span class="sk-block search-icon"></span>
			<span class="sk-block sk-line search-line"></span>
			<span class="sk-block search-submit"></span>
		</div>

		<div class="browse-controls">
			<div class="source-tabs">
				{#each { length: 2 } as _, index (index)}
					<span class="sk-block source-tab" class:active={index === 0}
					></span>
				{/each}
			</div>
			<div class="filters">
				<span class="sk-block filter-select"></span>
				<span class="sk-block filter-select"></span>
				<span class="sk-block reset-btn"></span>
			</div>
		</div>
	</div>

	<div class="results-bar">
		<span class="sk-block sk-line sk-line-sm" style="width: 148px"></span>
	</div>

	<div class="market-results-skeleton">
		<div class="grid">
			{#each { length: 8 } as _, index (index)}
				<MarketSkeleton />
			{/each}
		</div>
	</div>

	<div class="results-footer">
		<span class="sk-block sk-line sk-line-sm" style="width: 120px"></span>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.market-catalog-skeleton {
		container: market / inline-size;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: var(--bg-main);
	}

	.filter-panel-skeleton {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 20px var(--market-page-padding, 20px) 14px;
		background: var(--market-panel-bg);
		border-bottom: var(--border-width) solid var(--border);
		flex-shrink: 0;
	}

	.search-heading,
	.browse-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.heading-title {
		width: 168px;
		height: 20px;
	}

	.compatibility {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}

	.chip {
		width: 74px;
		height: 22px;
		border-radius: var(--border-radius-sm);
	}

	.chip.short {
		width: 52px;
	}

	.search-row-skeleton {
		display: flex;
		align-items: center;
		gap: 0;
		padding: 4px;
		background: var(--surface-input);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius-sm);
	}

	.search-icon {
		width: 18px;
		height: 18px;
		margin: 0 12px;
		flex-shrink: 0;
	}

	.search-line {
		flex: 1;
		height: 12px;
		margin: 8px 0;
	}

	.search-submit {
		width: 92px;
		height: 34px;
		margin-left: 6px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
	}

	.source-tabs {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-wrap: wrap;
	}

	.source-tab {
		width: 108px;
		height: 32px;
		border-radius: var(--border-radius-sm);
	}

	.filters {
		display: flex;
		align-items: flex-end;
		justify-content: flex-end;
		gap: 10px;
		flex: 1 1 320px;
		flex-wrap: wrap;
	}

	.filter-select {
		min-width: 0;
		flex: 1 1 140px;
		max-width: 180px;
		height: 34px;
		border-radius: var(--border-radius-sm);
	}

	.reset-btn {
		width: 34px;
		height: 32px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
	}

	.results-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 40px;
		padding: 0 var(--market-page-padding, 20px);
		flex-shrink: 0;
	}

	.market-results-skeleton {
		flex: 1;
		min-height: 0;
		margin: 0 var(--market-page-padding, 20px);
		overflow: hidden;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(
			auto-fill,
			minmax(min(var(--market-card-min-width, 280px), 100%), 1fr)
		);
		grid-auto-rows: calc(
			var(--market-row-height, 224px) - var(--market-grid-gap, 12px)
		);
		gap: var(--market-grid-gap, 12px);
		padding: 8px 0;
	}

	.results-footer {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 10px;
		padding: 10px var(--market-page-padding, 20px);
		min-height: 48px;
		box-sizing: border-box;
		flex-shrink: 0;
	}

	@container market (max-width: 700px) {
		.filter-panel-skeleton {
			padding: var(--market-page-padding-compact, 12px);
			gap: 10px;
		}

		.results-bar,
		.results-footer {
			padding-inline: var(--market-page-padding-compact, 12px);
		}

		.market-results-skeleton {
			margin-inline: var(--market-page-padding-compact, 12px);
		}

		.filters {
			width: 100%;
		}
	}
</style>
