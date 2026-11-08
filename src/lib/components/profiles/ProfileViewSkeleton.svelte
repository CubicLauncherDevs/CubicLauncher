<script lang="ts">
	import { t } from "$lib/i18n";
</script>

<div class="profile-view-skeleton sk-delayed" aria-hidden="true">
	<header class="profile-header">
		<span class="sk-block profile-title"></span>
		<span class="sk-block close-btn"></span>
	</header>

	<div class="profile-layout">
		<section class="profile-hero">
			<div class="hero-content">
				<div class="hero-identity">
					<span class="sk-block hero-avatar"></span>
					<div class="sk-stack hero-meta">
						<span class="sk-block hero-name"></span>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 32%"
						></span>
					</div>
					<div class="hero-actions">
						<span class="sk-block action-btn"></span>
						<span class="sk-block action-btn"></span>
					</div>
				</div>

				<div class="hero-stats">
					<div class="stat-card">
						<span class="sk-block sk-line" style="width: 34%"
						></span>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 68%"
						></span>
					</div>
					<div class="stat-card">
						<span class="sk-block sk-line" style="width: 44%"
						></span>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 56%"
						></span>
					</div>
					<div class="stat-card wide">
						<span class="sk-block sk-line" style="width: 52%"
						></span>
						<span
							class="sk-block sk-line sk-line-sm"
							style="width: 72%"
						></span>
					</div>
				</div>

				<div class="hero-skin-cape">
					<span class="sk-block skin-preview"></span>
				</div>
			</div>
		</section>

		<aside class="profile-list">
			<div class="add-card">
				<span class="sk-block sk-line" style="width: 58%"></span>
				<span class="sk-block sk-line sk-line-sm" style="width: 86%"
				></span>
				<span class="sk-block add-btn"></span>
			</div>

			<span class="sk-block list-title"></span>
			<div class="account-list">
				{#each { length: 3 } as _, index (index)}
					<div class="account-item">
						<span class="sk-block account-avatar"></span>
						<div class="sk-stack account-info">
							<span class="sk-block sk-line" style="width: 62%"
							></span>
							<span
								class="sk-block sk-line sk-line-sm"
								style="width: 38%"
							></span>
						</div>
						<span class="sk-block account-badge"></span>
					</div>
				{/each}
			</div>
		</aside>
	</div>
</div>
<span class="sk-sr-only" role="status">{t("common.loading")}</span>

<style>
	.profile-view-skeleton {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-main);
		z-index: 5;
	}

	.profile-header {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 28px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-card);
	}

	.profile-title {
		width: 140px;
		height: 16px;
	}

	.close-btn {
		width: 30px;
		height: 30px;
		border-radius: var(--border-radius-sm);
	}

	.profile-layout {
		display: flex;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.profile-hero {
		flex: 3;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		background: var(--bg-main);
	}

	.hero-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 20px;
		padding: 24px 28px;
		min-height: 0;
	}

	.hero-identity {
		display: flex;
		align-items: center;
		gap: 16px;
		flex-wrap: wrap;
	}

	.hero-avatar {
		width: 64px;
		height: 64px;
		border-radius: var(--border-radius);
		flex-shrink: 0;
	}

	.hero-meta {
		flex: 1;
		min-width: 160px;
		gap: 6px;
	}

	.hero-name {
		width: min(220px, 40%);
		height: 26px;
		border-radius: var(--border-radius-sm);
	}

	.hero-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-left: auto;
		flex-wrap: wrap;
	}

	.action-btn {
		width: 92px;
		height: 32px;
	}

	.hero-stats {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}

	.stat-card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 110px;
		padding: 14px 18px;
		box-sizing: border-box;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}

	.stat-card.wide {
		min-width: 160px;
		flex: 1;
	}

	.hero-skin-cape {
		width: 100%;
		flex: 1;
		min-height: 180px;
		display: flex;
		padding-top: 4px;
	}

	.skin-preview {
		width: 100%;
		height: 100%;
		min-height: 180px;
		border-radius: var(--border-radius);
	}

	.profile-list {
		flex: 2;
		min-width: 300px;
		max-width: 440px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: 20px;
		overflow-y: auto;
		background: var(--bg-sidebar);
		border-left: 1px solid var(--border);
		box-sizing: border-box;
	}

	.add-card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 16px;
		border: 1px dashed var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}

	.add-btn {
		width: 100%;
		height: 36px;
		margin-top: 4px;
	}

	.list-title {
		width: 120px;
		height: 12px;
	}

	.account-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.account-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
	}

	.account-avatar {
		width: 40px;
		height: 40px;
		border-radius: var(--border-radius-sm);
		flex-shrink: 0;
	}

	.account-info {
		flex: 1;
		gap: 6px;
	}

	.account-badge {
		width: 54px;
		height: 18px;
		border-radius: 999px;
		flex-shrink: 0;
	}

	@media (max-width: 900px) {
		.profile-layout {
			flex-direction: column;
			overflow-y: auto;
		}

		.profile-hero {
			flex: none;
			overflow: visible;
		}

		.hero-content {
			padding: 20px;
			gap: 18px;
		}

		.profile-list {
			flex: none;
			min-width: 0;
			max-width: none;
			border-left: none;
			border-top: 1px solid var(--border);
		}
	}
</style>
