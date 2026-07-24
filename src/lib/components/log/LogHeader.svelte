<script lang="ts">
	interface Props {
		instanceName: string;
		totalLines: number;
		isAtBottom: boolean;
		uploading: boolean;
		onClear: () => void;
		onCopy: () => void;
		onUpload: () => void;
		onScrollBottom: () => void;
	}

	let {
		instanceName,
		totalLines,
		isAtBottom,
		uploading,
		onClear,
		onCopy,
		onUpload,
		onScrollBottom,
	}: Props = $props();
</script>

<div class="log-header">
	<div class="log-title">
		<span class="log-dot" class:alive={totalLines > 0}></span>
		<span>{instanceName}</span>
		{#if totalLines > 0}
			<span class="log-count">{totalLines}</span>
		{/if}
	</div>
	<div class="log-toolbar">
		<button
			type="button"
			class="toolbar-btn"
			onclick={onClear}
			disabled={totalLines === 0}
			title="Clear log"
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<polyline points="3 6 5 6 21 6" /><path
					d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
				/>
			</svg>
		</button>
		<button
			type="button"
			class="toolbar-btn"
			onclick={onCopy}
			disabled={totalLines === 0}
			title="Copy log"
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path
					d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
				/>
			</svg>
		</button>
		<button
			type="button"
			class="toolbar-btn"
			onclick={onUpload}
			disabled={totalLines === 0 || uploading}
			title={uploading ? "Uploading..." : "Upload to mclo.gs"}
		>
			{#if uploading}
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					class="spin"
				>
					<line x1="12" y1="2" x2="12" y2="6" /><line
						x1="12"
						y1="18"
						x2="12"
						y2="22"
					/><line x1="4.93" y1="4.93" x2="7.76" y2="7.76" /><line
						x1="16.24"
						y1="16.24"
						x2="19.07"
						y2="19.07"
					/><line x1="2" y1="12" x2="6" y2="12" /><line
						x1="18"
						y1="12"
						x2="22"
						y2="12"
					/><line
						x1="4.93"
						y1="19.07"
						x2="7.76"
						y2="16.24"
					/><line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
				</svg>
			{:else}
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
					/><polyline points="17 8 12 3 7 8" /><line
						x1="12"
						y1="3"
						x2="12"
						y2="15"
					/>
				</svg>
			{/if}
		</button>
		<button
			type="button"
			class="toolbar-btn"
			class:active={isAtBottom}
			onclick={onScrollBottom}
			title="Auto-scroll"
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<line x1="12" y1="5" x2="12" y2="19" /><polyline
					points="19 12 12 19 5 12"
				/>
			</svg>
		</button>
	</div>
</div>

<style>
	.log-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 14px;
		background: var(--bg-card, #111);
		border-bottom: 1px solid var(--border, #222);
		flex-shrink: 0;
	}

	.log-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 0.62rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 1px;
		color: var(--text-tertiary, #888);
	}

	.log-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #546e7a;
		transition: all 0.3s ease;
	}

	.log-dot.alive {
		background: var(--color-status-started, #66bb6a);
		box-shadow: 0 0 8px rgba(102, 187, 106, 0.5);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.log-count {
		background: rgba(255, 255, 255, 0.08);
		padding: 1px 6px;
		border-radius: 8px;
		font-size: 0.55rem;
		color: var(--text-secondary, #666);
	}

	.log-toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.toolbar-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary, #666);
		width: 26px;
		height: 26px;
		border-radius: 4px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s ease;
		font-size: 0.75rem;
	}

	.toolbar-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-primary, #ccc);
	}

	.toolbar-btn.active {
		color: var(--color-success, #81c784);
		background: rgba(var(--color-success-rgb), 0.12);
	}

	.toolbar-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
