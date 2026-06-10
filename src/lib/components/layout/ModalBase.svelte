<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import type { Snippet } from "svelte";

	let {
		open = $bindable(),
		title,
		onclose,
		children,
		footer,
	} = $props<{
		open: boolean;
		title?: string;
		onclose?: () => void;
		children?: Snippet;
		footer?: Snippet;
	}>();

	function close() {
		open = false;
		onclose?.();
	}
</script>

{#if open}
	<div
		class="modal-overlay"
		onclick={close}
		onkeydown={(e) => e.key === "Escape" && close()}
		role="presentation"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="-1"
			transition:fly={{ y: 20, duration: 250 }}
		>
			<div class="modal-header">
				{#if title}
					<span class="modal-title">{title}</span>
				{/if}
				<button
					type="button"
					class="action-btn"
					onclick={close}
					aria-label="Close"
				>
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						stroke-linejoin="round"
						><line x1="18" y1="6" x2="6" y2="18"></line><line
							x1="6"
							y1="6"
							x2="18"
							y2="18"
						></line></svg
					>
				</button>
			</div>

			<div class="modal-body">
				{@render children?.()}
			</div>

			{#if footer}
				<div class="modal-footer">
					{@render footer()}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.75);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
	}

	.modal {
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		width: min(400px, 90vw);
		max-height: 90vh;
		overflow-y: auto;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
		box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
	}

	:global(.modal::-webkit-scrollbar) {
		width: 6px;
	}

	:global(.modal::-webkit-scrollbar-thumb) {
		background: var(--border);
		border-radius: 6px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.modal-title {
		font-size: 1rem;
		font-weight: 700;
		letter-spacing: 0.5px;
		color: var(--text-primary);
	}

	.modal-body {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
	}
</style>
