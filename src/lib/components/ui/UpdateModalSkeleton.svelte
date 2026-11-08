<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="update-content-skeleton sk-delayed" aria-hidden="true">
	<div class="update-heading">
		<span class="sk-block status-icon"></span>
		<div class="heading-text">
			<span class="sk-block sk-line heading-title"></span>
			<span class="sk-block sk-line sk-line-sm heading-subtitle"></span>
		</div>
	</div>

	<div class="versions">
		<div class="version-field">
			<span class="sk-block sk-line sk-line-sm" style="width: 58%"></span>
			<span class="sk-block version-value"></span>
		</div>
		<span class="sk-block version-arrow"></span>
		<div class="version-field">
			<span class="sk-block sk-line sk-line-sm" style="width: 52%"></span>
			<span class="sk-block version-value new"></span>
		</div>
	</div>

	<div class="release-notes">
		<span class="sk-block sk-line sk-line-sm" style="width: 28%"></span>
		<div class="notes-content">
			{#each { length: 3 } as _, index (index)}
				<span
					class="sk-block sk-line"
					style={`width: ${index === 2 ? 54 : 94}%`}
				></span>
			{/each}
		</div>
	</div>

	<div class="update-actions">
		<span class="sk-block action secondary"></span>
		<span class="sk-block action primary"></span>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.update-content-skeleton {
		display: flex;
		flex-direction: column;
		gap: 20px;
		min-width: 0;
	}

	.update-heading {
		display: flex;
		align-items: flex-start;
		gap: 14px;
	}

	.status-icon {
		width: 48px;
		height: 48px;
		border-radius: var(--border-radius);
		flex-shrink: 0;
	}

	.heading-text {
		display: flex;
		flex-direction: column;
		gap: 8px;
		flex: 1;
		padding-top: 4px;
	}

	.heading-title {
		width: min(260px, 70%);
		height: 18px;
	}

	.heading-subtitle {
		width: min(340px, 88%);
		height: 12px;
	}

	.versions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 18px;
		padding: 14px;
		background: var(--surface-input);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
	}

	.version-field {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 96px;
	}

	.version-value {
		width: 72px;
		height: 16px;
	}

	.version-field:last-child .version-value {
		width: 86px;
	}

	.version-arrow {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}

	.release-notes {
		display: flex;
		flex-direction: column;
		gap: 12px;
		border-top: 1px solid var(--border);
		padding-top: 14px;
	}

	.notes-content {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.update-actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 10px;
		width: 100%;
	}

	.action {
		width: 112px;
		height: 34px;
		border-radius: var(--border-radius-sm);
	}

	.action.primary {
		width: 148px;
	}

	@media (max-width: 520px) {
		.status-icon {
			width: 40px;
			height: 40px;
		}

		.action {
			width: 100%;
		}

		.action.primary {
			width: 100%;
		}
	}
</style>
