<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="instance-view-skeleton sk-delayed" aria-hidden="true">
	<div class="instance-header-skeleton">
		<div class="header-bg"></div>
		<div class="header-content">
			<div class="title-row">
				<div class="title-left">
					<span class="sk-block instance-icon"></span>
					<span class="sk-block instance-title"></span>
				</div>
				<div class="actions-row">
					{#each { length: 4 } as _, index (index)}
						<span class="sk-block action-btn"></span>
					{/each}
				</div>
			</div>
			<div class="extension-row">
				<div class="extension-left">
					<span class="sk-block meta-chip"></span>
					<span class="sk-block meta-status"></span>
					<span class="sk-block meta-log"></span>
				</div>
				<span class="sk-block launch-btn"></span>
			</div>
			<div class="details-row">
				<span class="sk-block modpack-chip"></span>
				<span class="sk-block path-row"></span>
				<span class="sk-block playtime"></span>
			</div>
		</div>
	</div>

	<div class="instance-body-skeleton">
		<div class="nav-card-skeleton">
			<div class="nav-card-header">
				<span class="sk-block sk-line sk-line-sm" style="width: 34%"
				></span>
			</div>
			{#each { length: 2 } as _, index (index)}
				<div class="nav-row">
					<span class="sk-block nav-icon"></span>
					<span class="sk-block sk-line" style="width: 52%"></span>
					<span class="sk-block nav-chevron"></span>
				</div>
			{/each}
		</div>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.instance-view-skeleton {
		position: relative;
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.instance-header-skeleton {
		position: relative;
		flex-shrink: 0;
		height: 180px;
	}

	.header-bg {
		position: absolute;
		inset: 0;
		bottom: -40px;
		background: var(--bg-sidebar);
		border-bottom: 1px solid var(--border);
		-webkit-mask-image: linear-gradient(
			black 0%,
			black 30%,
			transparent 80%
		);
		mask-image: linear-gradient(black 0%, black 30%, transparent 80%);
	}

	.header-content {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 24px 24px 16px;
		box-sizing: border-box;
	}

	.title-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.title-left {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.instance-icon {
		width: 36px;
		height: 36px;
		border-radius: 6px;
		flex-shrink: 0;
	}

	.instance-title {
		width: min(280px, 46vw);
		height: 30px;
		border-radius: var(--border-radius-sm);
	}

	.actions-row {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}

	.action-btn {
		width: 76px;
		height: 28px;
	}

	.extension-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.extension-left {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		flex: 1;
	}

	.meta-chip {
		width: 92px;
		height: 22px;
		border-radius: 5px;
	}

	.meta-status {
		width: 72px;
		height: 16px;
	}

	.meta-log {
		width: min(240px, 24vw);
		height: 12px;
	}

	.launch-btn {
		width: 145px;
		height: 38px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
	}

	.details-row {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-top: auto;
		min-width: 0;
	}

	.modpack-chip {
		width: 120px;
		height: 24px;
		border-radius: var(--border-radius-sm);
	}

	.path-row {
		width: min(280px, 30vw);
		height: 14px;
	}

	.playtime {
		width: 96px;
		height: 14px;
	}

	.instance-body-skeleton {
		position: relative;
		flex: 1;
		min-height: 0;
		padding: 24px;
		box-sizing: border-box;
	}

	.nav-card-skeleton {
		position: absolute;
		bottom: 24px;
		left: 24px;
		width: 340px;
		max-width: calc(100% - 48px);
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
		overflow: hidden;
	}

	.nav-card-header {
		padding: 9px 14px;
	}

	.nav-row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 40px;
		padding: 10px 14px;
		box-sizing: border-box;
	}

	.nav-icon {
		width: 20px;
		height: 20px;
		flex-shrink: 0;
	}

	.nav-chevron {
		width: 16px;
		height: 16px;
		margin-left: auto;
		flex-shrink: 0;
	}

	@media (max-width: 700px) {
		.instance-header-skeleton {
			height: 164px;
		}

		.header-content {
			padding: 16px;
		}

		.instance-body-skeleton {
			padding: 16px;
		}

		.nav-card-skeleton {
			bottom: 16px;
			left: 16px;
			max-width: calc(100% - 32px);
		}
	}

	@media (max-width: 520px) {
		.actions-row {
			display: none;
		}

		.meta-log,
		.playtime {
			display: none;
		}
	}
</style>
