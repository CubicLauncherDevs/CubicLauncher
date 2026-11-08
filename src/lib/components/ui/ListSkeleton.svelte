<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		rows = 6,
		label = t("common.loading"),
		rowHeight = "84px",
		iconSize = "54px",
		showDetails = true,
		detailRows = 5,
		actionCount = 2,
		wideFirstAction = false,
	}: {
		rows?: number;
		label?: string;
		rowHeight?: string;
		iconSize?: string;
		showDetails?: boolean;
		detailRows?: number;
		actionCount?: number;
		wideFirstAction?: boolean;
	} = $props();
</script>

<div class="list-skeleton sk-delayed" aria-hidden="true">
	<div class="list-skeleton-rows">
		{#each { length: rows } as _, index (index)}
			<div class="list-skeleton-row" style:min-height={rowHeight}>
				<span
					class="sk-block"
					style:width={iconSize}
					style:height={iconSize}
					style:border-radius="var(--border-radius-sm)"
				></span>
				<div class="sk-stack list-skeleton-text">
					<span class="sk-block sk-line" style="width: 55%"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 34%"
					></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 44%"
					></span>
				</div>
			</div>
		{/each}
	</div>
	{#if showDetails}
		<aside class="list-skeleton-details">
			<span class="sk-block sk-line-lg" style="width: 50%"></span>
			{#each { length: detailRows } as _, index (index)}
				<div class="sk-stack list-skeleton-field">
					<span class="sk-block sk-line sk-line-sm" style="width: 30%"
					></span>
					<span class="sk-block sk-line" style="width: 72%"></span>
				</div>
			{/each}
			<div class="list-skeleton-actions">
				{#each { length: actionCount } as _, index (index)}
					<span
						class="sk-block list-skeleton-action"
						class:wide={wideFirstAction && index === 0}
					></span>
				{/each}
			</div>
		</aside>
	{/if}
</div>
<span class="sk-sr-only" role="status">{label}</span>

<style>
	.list-skeleton {
		display: grid;
		grid-template-columns: minmax(240px, 1fr) minmax(240px, 320px);
		gap: 20px;
		align-items: start;
	}

	.list-skeleton-rows {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}

	.list-skeleton-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px;
		box-sizing: border-box;
	}

	.list-skeleton-text {
		flex: 1;
		gap: 6px;
	}

	.list-skeleton-details {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 18px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}

	.list-skeleton-field {
		gap: 6px;
	}

	.list-skeleton-actions {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
		margin-top: 4px;
	}

	.list-skeleton-action {
		height: 32px;
	}

	.list-skeleton-action.wide {
		grid-column: 1 / -1;
	}

	@media (max-width: 700px) {
		.list-skeleton {
			grid-template-columns: 1fr;
		}

		.list-skeleton-details {
			display: none;
		}
	}
</style>
