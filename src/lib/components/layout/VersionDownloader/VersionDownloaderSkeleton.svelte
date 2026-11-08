<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="version-catalog-skeleton sk-delayed" aria-hidden="true">
	<div class="catalog-tabs">
		{#each { length: 4 } as _, index (index)}
			<div class="catalog-tab" class:active={index === 0}>
				<span class="sk-block tab-icon"></span>
				<span class="sk-block tab-label"></span>
			</div>
		{/each}
	</div>

	<div class="catalog-panel">
		<div class="source-heading">
			<span class="sk-block source-icon"></span>
			<div class="source-text">
				<span class="sk-block sk-line" style="width: 128px"></span>
				<span class="sk-block sk-line sk-line-sm" style="width: 240px"
				></span>
			</div>
		</div>

		<div class="catalog-filters">
			<div class="filter-row">
				<div class="filter-field mc-filter">
					<span class="sk-block sk-line sk-line-sm" style="width: 46%"
					></span>
					<span class="sk-block filter-input"></span>
				</div>
				<div class="filter-field search-filter">
					<span class="sk-block sk-line sk-line-sm" style="width: 26%"
					></span>
					<span class="sk-block filter-input"></span>
				</div>
			</div>
		</div>

		<div class="results-heading">
			<span class="sk-block sk-line sk-line-sm" style="width: 118px"
			></span>
		</div>

		<div class="catalog-results">
			{#each { length: 6 } as _, index (index)}
				<div class="version-row">
					<span class="sk-block row-icon"></span>
					<div class="version-info">
						<div class="version-title">
							<span class="sk-block sk-line" style="width: 38%"
							></span>
							<span class="sk-block version-badge"></span>
						</div>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 28%"
						></span>
					</div>
					<span class="sk-block download-btn"></span>
				</div>
			{/each}
		</div>

		<div class="catalog-footer">
			<span class="sk-block footer-icon"></span>
			<span class="sk-block sk-line sk-line-sm" style="width: 220px"
			></span>
		</div>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.version-catalog-skeleton {
		display: flex;
		flex-direction: column;
		height: min(640px, 72vh);
		min-height: 0;
		gap: 22px;
	}

	.catalog-tabs {
		display: flex;
		flex-shrink: 0;
		gap: 4px;
		border-bottom: 1px solid var(--border);
	}

	.catalog-tab {
		display: flex;
		flex: 1 0 auto;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 14px 12px;
	}

	.tab-icon {
		width: 18px;
		height: 18px;
	}

	.tab-label {
		width: 54px;
		height: 11px;
	}

	.catalog-panel {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		gap: 18px;
	}

	.source-heading {
		display: flex;
		align-items: center;
		gap: 16px;
		flex-shrink: 0;
	}

	.source-icon {
		width: 64px;
		height: 64px;
		border-radius: 16px;
		flex-shrink: 0;
	}

	.source-text {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.catalog-filters {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 16px;
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		flex-shrink: 0;
	}

	.filter-row {
		display: flex;
		gap: 14px;
		align-items: flex-end;
	}

	.filter-field {
		display: flex;
		flex-direction: column;
		gap: 7px;
	}

	.mc-filter {
		width: 190px;
		flex-shrink: 0;
	}

	.search-filter {
		flex: 1;
	}

	.filter-input {
		width: 100%;
		height: 34px;
		border-radius: var(--border-radius-sm);
	}

	.results-heading {
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}

	.catalog-results {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 100px;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}

	.version-row {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 78px;
		flex-shrink: 0;
		padding: 12px 16px;
		box-sizing: border-box;
		border-bottom: 1px solid var(--border);
	}

	.row-icon {
		width: 38px;
		height: 38px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
	}

	.version-info {
		display: flex;
		flex-direction: column;
		gap: 7px;
		flex: 1;
		min-width: 0;
	}

	.version-title {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.version-badge {
		width: 44px;
		height: 14px;
		border-radius: 999px;
	}

	.download-btn {
		width: 104px;
		height: 32px;
		margin-left: auto;
		flex-shrink: 0;
	}

	.catalog-footer {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		flex-shrink: 0;
	}

	.footer-icon {
		width: 15px;
		height: 15px;
	}

	@media (max-width: 700px) {
		.version-catalog-skeleton {
			gap: 14px;
			height: 72vh;
		}

		.catalog-panel {
			gap: 12px;
		}

		.source-icon {
			width: 50px;
			height: 50px;
			border-radius: 12px;
		}

		.catalog-filters {
			padding: 12px;
		}

		.mc-filter {
			width: 130px;
		}

		.filter-row {
			gap: 10px;
		}

		.version-row {
			padding: 10px;
		}
	}

	@media (max-width: 380px) {
		.filter-row {
			flex-direction: column;
			align-items: stretch;
		}

		.mc-filter {
			width: 100%;
		}
	}
</style>
