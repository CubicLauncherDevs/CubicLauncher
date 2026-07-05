<script lang="ts">
	import { t } from "$lib/i18n";
	import type { ModDownloadInfo } from "$lib/api/cubicApi";
	import Loading from "$lib/icons/Loading.svelte";

	let {
		resolvingDeps = false,
		downloading = false,
		downloadQueue = [] as ModDownloadInfo[],
		onBack,
		onConfirmDownload,
	}: {
		resolvingDeps: boolean;
		downloading: boolean;
		downloadQueue: ModDownloadInfo[];
		onBack: () => void;
		onConfirmDownload: () => Promise<void>;
	} = $props();
</script>

<div class="dm-review">
	<div class="dm-review-header">
		<div>
			<span class="dm-section-label"
				>{t("instanceView.downloadMods.sectionLabel")}</span
			>
			<h2 class="dm-review-title">
				{t("instanceView.downloadMods.reviewTitle")}
			</h2>
		</div>
		<button
			type="button"
			class="dm-back-btn"
			onclick={onBack}
			disabled={downloading}
		>
			{t("instanceView.downloadMods.back")}
		</button>
	</div>

	<div class="dm-review-body">
		{#if resolvingDeps}
			<div class="dm-center-state">
				<Loading />
				<p>{t("instanceView.downloadMods.resolvingDeps")}</p>
			</div>
		{:else if downloadQueue.length === 0}
			<div class="dm-center-state">
				<p>{t("instanceView.downloadMods.allInstalled")}</p>
				<span style="font-size:0.75rem; opacity:0.5;"
					>{t("instanceView.downloadMods.allInstalledSub")}</span
				>
			</div>
		{:else}
			<div class="dm-queue-box">
				<p class="dm-queue-subtitle">
					{downloadQueue.length}
					{downloadQueue.length === 1
						? t("instanceView.downloadMods.file_one")
						: t("instanceView.downloadMods.file_other")} para descargar:
				</p>
				<div class="dm-queue-list">
					{#each downloadQueue as item (item.filename)}
						<div class="dm-queue-item">
							{#if item.iconUrl}
								<img
									src={item.iconUrl}
									alt=""
									class="dm-queue-icon-img"
								/>
							{:else}
								<span class="dm-queue-icon">📦</span>
							{/if}
							<div class="dm-queue-item-info">
								{#if item.projectTitle}
									<span class="dm-queue-title"
										>{item.projectTitle}</span
									>
								{/if}
								<span class="dm-queue-filename"
									>{item.filename}</span
								>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<div class="dm-review-footer">
				<span class="dm-review-count">
					<strong>{downloadQueue.length}</strong>
					{downloadQueue.length !== 1
						? t("instanceView.downloadMods.file_other")
						: t("instanceView.downloadMods.file_one")}
				</span>
				<button
					type="button"
					class="dm-primary-btn"
					onclick={onConfirmDownload}
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
	.dm-review {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 28px 32px;
	}
	.dm-review-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 24px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}
	.dm-review-title {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.dm-review-body {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dm-section-label {
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		color: var(--text-secondary);
		margin-bottom: 8px;
		display: block;
	}
	.dm-queue-box {
		flex: 1;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 16px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dm-queue-subtitle {
		font-size: 0.78rem;
		color: var(--text-secondary);
		margin: 0 0 14px 0;
	}
	.dm-queue-list {
		flex: 1;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 8px;
		align-content: flex-start;
	}
	.dm-queue-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		padding: 10px 12px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.dm-queue-item-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
		gap: 2px;
	}
	.dm-queue-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-queue-icon {
		font-size: 1rem;
		opacity: 0.6;
	}
	.dm-queue-filename {
		font-size: 0.8rem;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dm-review-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 16px;
		padding: 14px 16px;
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
	}
	.dm-review-count {
		font-size: 0.85rem;
		color: var(--text-secondary);
	}
	.dm-review-count strong {
		color: var(--text-primary);
		font-size: 1.1rem;
	}
	.dm-primary-btn {
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
	.dm-primary-btn:hover:not(:disabled) {
		filter: brightness(0.9);
	}
	.dm-primary-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}

	.dm-back-btn {
		background: transparent;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 7px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-size: 0.82rem;
		transition: all 0.15s;
	}
	.dm-back-btn:hover:not(:disabled) {
		background: var(--bg-item-active);
		color: var(--text-primary);
	}
	.dm-back-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.dm-center-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 240px;
		gap: 14px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.dm-queue-icon-img {
		width: 24px;
		height: 24px;
		border-radius: var(--border-radius-sm);
		object-fit: cover;
		flex-shrink: 0;
	}

	:global(.dm-spinning) {
		animation: spin 0.8s linear infinite;
		will-change: transform;
	}
	:global {
		@keyframes spin {
			to {
				transform: rotate(360deg);
			}
		}
	}
</style>
