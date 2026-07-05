<script lang="ts">
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { t } from "$lib/i18n";

	let {
		showPicker = $bindable(false),
		allScreenshots = [] as string[],
		onSelect,
	}: {
		showPicker: boolean;
		allScreenshots: string[];
		onSelect: (path: string) => void;
	} = $props();
</script>

{#if showPicker}
	<div
		class="screenshot-picker-overlay"
		role="button"
		tabindex="0"
		onclick={() => (showPicker = false)}
		onkeydown={(e) => e.key === "Escape" && (showPicker = false)}
	>
		<div
			class="screenshot-picker-modal"
			role="dialog"
			aria-modal="true"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="picker-header">
				<h3>{t("instanceView.pickBannerTitle")}</h3>
				<button
					type="button"
					class="close-btn"
					onclick={() => (showPicker = false)}>✕</button
				>
			</div>
			<div class="picker-content">
				{#if allScreenshots.length === 0}
					<div class="empty-picker">
						{t("instanceView.noScreenshots")}
					</div>
				{:else}
					<div class="picker-grid">
						{#each allScreenshots as path (path)}
							<button
								type="button"
								class="picker-item"
								onclick={() => onSelect(path)}
							>
								<img
									src={convertFileSrc(path)}
									alt="Screenshot"
									loading="lazy"
								/>
							</button>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.screenshot-picker-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(var(--backdrop-blur-viewer, 2px));
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		animation: fadeIn 0.2s ease-out;
	}

	.screenshot-picker-modal {
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		width: 600px;
		max-width: 90vw;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
		animation: scaleUp 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.picker-header {
		padding: 16px 20px;
		border-bottom: 1px solid var(--border);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.picker-header h3 {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 1px;
	}

	.close-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		font-size: 1.2rem;
		cursor: pointer;
		transition: color 0.2s;
	}

	.close-btn:hover {
		color: var(--text-primary);
	}

	.picker-content {
		flex: 1;
		overflow-y: auto;
		padding: 20px;
	}

	.picker-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 12px;
	}

	.picker-item {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		padding: 0;
		aspect-ratio: 16/9;
		cursor: pointer;
		transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
	}

	.picker-item img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		opacity: 0.8;
		transition: all 0.25s;
	}

	.picker-item:hover {
		border-color: var(--accent);
		box-shadow: 0 10px 20px rgba(0, 0, 0, 0.3);
	}

	.picker-item:hover img {
		opacity: 1;
	}

	.empty-picker {
		text-align: center;
		padding: 40px;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes scaleUp {
		from {
			opacity: 0;
			transform: scale(0.95);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}
</style>
