<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="create-skeleton sk-delayed" aria-hidden="true">
	<div class="create-skeleton-choices">
		{#each { length: 2 } as _, index (index)}
			<div class="create-skeleton-choice">
				<span
					class="sk-block"
					style="width: 44px; height: 44px; border-radius: var(--border-radius-lg, 8px);"
				></span>
				<div class="sk-stack create-skeleton-copy">
					<span class="sk-block sk-line" style="width: 62%"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 92%"
					></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 74%"
					></span>
				</div>
			</div>
		{/each}
	</div>
	<div class="create-skeleton-imports">
		{#each { length: 2 } as _, index (index)}
			<div class="create-skeleton-import">
				<span
					class="sk-block"
					style="width: 32px; height: 32px; flex-shrink: 0;"
				></span>
				<div class="sk-stack create-skeleton-import-copy">
					<span class="sk-block sk-line" style="width: 58%"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 82%"
					></span>
				</div>
			</div>
		{/each}
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.create-skeleton {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-height: 320px;
		box-sizing: border-box;
	}

	.create-skeleton-choices,
	.create-skeleton-imports {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 16px;
	}

	.create-skeleton-choice {
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: 20px;
		min-width: 0;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
		background: var(--bg-card);
	}

	.create-skeleton-copy {
		gap: 6px;
	}

	.create-skeleton-imports {
		gap: 10px;
		padding-top: 16px;
		border-top: 1px solid var(--border);
	}

	.create-skeleton-import {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px;
		min-width: 0;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
	}

	.create-skeleton-import-copy {
		flex: 1;
		gap: 6px;
	}

	@media (max-width: 500px) {
		.create-skeleton-choices,
		.create-skeleton-imports {
			grid-template-columns: 1fr;
		}

		.create-skeleton-choice {
			flex-direction: row;
			align-items: center;
			gap: 12px;
			padding: 14px;
		}
	}
</style>
