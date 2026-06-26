<script lang="ts">
	import type { ModDownloadInfo } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";
	import Cubic from "$lib/icons/Cubic.svelte";
	import Loading from "$lib/icons/Loading.svelte";

	let {
		downloadQueue = $bindable(),
		i18nPrefix,
		downloading = $bindable(),
		// eslint-disable-next-line
		mode = $bindable(),
		confirmDownload = $bindable(),
	}: {
		downloadQueue: ModDownloadInfo[];
		i18nPrefix: string;
		downloading: boolean;
		mode: string;
		confirmDownload: () => void;
	} = $props();
</script>

<div class="rp-review">
	<div class="rp-review-header">
		<div>
			<span class="rp-section-label"
				>{t(i18nPrefix + ".sectionLabel")}</span
			>
			<h2 class="rp-review-title">
				{t("instanceView.downloadMods.reviewTitle")}
			</h2>
		</div>
		<button
			type="button"
			class="rp-back-btn"
			onclick={() => (mode = "browse")}
			disabled={downloading}
		>
			← {t("instanceView.downloadMods.back")}
		</button>
	</div>

	<div class="rp-review-body">
		{#if downloadQueue.length === 0}
			<div class="rp-center-state">
				<p>{t(i18nPrefix + ".noSelection")}</p>
			</div>
		{:else}
			<div class="rp-queue-box">
				<p class="rp-queue-subtitle">
					{downloadQueue.length}
					{downloadQueue.length === 1
						? t("instanceView.downloadMods.file_one")
						: t("instanceView.downloadMods.file_other")}
					{t(i18nPrefix + ".toDownload")}
				</p>
				<div class="rp-queue-list">
					{#each downloadQueue as item (item.filename)}
						<div class="rp-queue-item">
							{#if item.iconUrl}
								<img
									src={item.iconUrl}
									alt=""
									class="rp-queue-icon-img"
								/>
							{:else}
								<Cubic width="24" height="24" />
							{/if}
							<div class="rp-queue-item-info">
								{#if item.projectTitle}
									<span class="rp-queue-title"
										>{item.projectTitle}</span
									>
								{/if}
								<span class="rp-queue-filename"
									>{item.filename}</span
								>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<div class="rp-review-footer">
				<span class="rp-review-count">
					<strong>{downloadQueue.length}</strong>
					{downloadQueue.length !== 1
						? t("instanceView.downloadMods.file_other")
						: t("instanceView.downloadMods.file_one")}
				</span>
				<button
					type="button"
					class="rp-primary-btn"
					onclick={confirmDownload}
					disabled={downloading}
				>
					{#if downloading}
						<Loading />
						{t("instanceView.downloadMods.downloading")}
					{:else}
						{t("instanceView.downloadMods.confirmDownload")}
					{/if}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.rp-back-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		transition: all 0.15s;
	}
	.rp-back-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}
	.rp-back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.rp-primary-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px 18px;
		background: var(--accent);
		color: var(--bg-main);
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		font-weight: 700;
		letter-spacing: 0.3px;
		transition: all 0.15s;
	}
	.rp-primary-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}
	.rp-primary-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
	.rp-primary-btn.rp-btn-remove {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary);
		border: 1px solid var(--border);
	}
	.rp-primary-btn.rp-btn-remove:hover:not(:disabled) {
		background: rgba(255, 68, 68, 0.12);
		color: #ff6b6b;
		border-color: rgba(255, 68, 68, 0.3);
	}
	.rp-review-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 16px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.rp-review-count {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.rp-review-count strong {
		color: var(--text-primary);
		font-size: 1.1rem;
	}
	.rp-review-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.rp-queue-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.rp-review-title {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.rp-queue-box {
		flex: 1;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.rp-queue-item-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		gap: 2px;
	}
	.rp-queue-list {
		flex: 1;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 8px;
		align-content: flex-start;
	}
	.rp-queue-subtitle {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0 0 14px 0;
	}
	.rp-queue-filename {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rp-review {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 28px 32px;
		backdrop-filter: blur(5px);
	}
	.rp-review-body {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.rp-queue-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rp-queue-icon {
		font-size: 1rem;
		opacity: 0.6;
	}
	.rp-queue-icon-img {
		width: 24px;
		height: 24px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
		flex-shrink: 0;
	}
</style>
