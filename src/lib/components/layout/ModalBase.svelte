<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import type { Snippet } from "svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";

	let {
		open = $bindable(),
		title,
		width,
		onclose,
		children,
		footer,
	} = $props<{
		open: boolean;
		title?: string;
		width?: string;
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
			style={width ? `width: min(${width}, 90vw)` : undefined}
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
					<CloseIcon size={20} />
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
		background: var(--bg-overlay, rgba(0, 0, 0, 0.75));
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
	}

	.modal {
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: var(--border-radius, 8px);
		width: min(400px, 90vw);
		max-height: 90vh;
		overflow-y: auto;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
		box-shadow: var(--shadow-lg, 0 20px 40px rgba(0, 0, 0, 0.4));
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
