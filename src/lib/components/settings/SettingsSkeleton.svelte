<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="qm-root settings-skeleton sk-delayed" aria-hidden="true">
	<div class="qm-header">
		<span class="sk-block header-title"></span>
		<span class="sk-block close-btn"></span>
	</div>

	<div class="qm-tabs">
		{#each { length: 3 } as _, index (index)}
			<span class="sk-block tab"></span>
		{/each}
	</div>

	<div class="qm-scroll">
		{#each { length: 3 } as _, group (group)}
			<div class="section-group">
				<div class="section-header">
					<span class="sk-block section-icon"></span>
					<span class="sk-block sk-line sk-line-sm" style="width: 38%"
					></span>
					<span class="sk-block section-chevron"></span>
				</div>
				<div class="section-content">
					{#each { length: group === 0 ? 3 : 2 } as _, field (field)}
						<div class="field">
							<span
								class="sk-block sk-line sk-line-sm"
								style="width: 28%"
							></span>
							{#if field === 0}
								<span class="sk-block input"></span>
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

	<div class="qm-footer">
		<span class="sk-block version"></span>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.settings-skeleton {
		height: 100%;
	}

	.header-title {
		width: 148px;
		height: 18px;
	}

	.close-btn {
		width: 22px;
		height: 22px;
	}

	.qm-tabs {
		gap: 6px;
	}

	.tab {
		flex: 1;
		height: 30px;
		margin: 6px 0;
		border-radius: var(--border-radius-sm);
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

	.input {
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

	.qm-footer {
		padding: 15px 20px;
	}

	.version {
		width: 128px;
		height: 10px;
	}

	@media (max-width: 520px) {
		.header-title {
			width: 110px;
		}

		.qm-scroll {
			padding: 10px;
		}
	}
</style>
