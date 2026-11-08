<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="qm-root instance-drawer-skeleton sk-delayed" aria-hidden="true">
	<div class="qm-header">
		<span class="sk-block header-title"></span>
		<span class="sk-block close-btn"></span>
	</div>

	<div class="qm-scroll">
		{#each { length: 4 } as _, group (group)}
			<div class="section-group">
				<div class="section-header">
					<span class="sk-block section-icon"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 42%"
					></span>
					<span class="sk-block section-chevron"></span>
				</div>
				<div class="section-content">
					{#each { length: group < 2 ? 3 : 2 } as _, field (field)}
						<div class="field">
							<span
								class="sk-block sk-line sk-line-sm"
								style="width: 30%"
							></span>
							{#if field === 0}
								<span class="sk-block input"></span>
							{:else if field === 1}
								<span class="sk-block select"></span>
							{:else}
								<span class="sk-block toggle-row"></span>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/each}
	</div>

	<div class="save-footer">
		<span class="sk-block save-btn"></span>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.instance-drawer-skeleton {
		height: 100%;
	}

	.header-title {
		width: 160px;
		height: 18px;
	}

	.close-btn {
		width: 22px;
		height: 22px;
	}

	.qm-scroll {
		padding: 12px 16px;
		box-sizing: border-box;
	}

	.section-group {
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		margin-bottom: 16px;
		overflow: hidden;
		background: var(--bg-card);
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px;
		border-bottom: 1px solid var(--border);
	}

	.section-icon {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}

	.section-chevron {
		width: 14px;
		height: 14px;
		margin-left: auto;
		flex-shrink: 0;
	}

	.section-content {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 12px 10px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.input,
	.select {
		width: 100%;
		height: 34px;
	}

	.toggle-row {
		width: 100%;
		height: 22px;
	}

	.save-footer {
		padding: 12px 20px;
		border-top: 1px solid var(--border-color);
	}

	.save-btn {
		width: 100%;
		height: 38px;
	}

	@media (max-width: 520px) {
		.header-title {
			width: 120px;
		}

		.qm-scroll {
			padding: 10px;
		}
	}
</style>
