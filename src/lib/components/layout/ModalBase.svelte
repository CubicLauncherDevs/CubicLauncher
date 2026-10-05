<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import type { Snippet } from "svelte";
	import CloseIcon from "$lib/icons/CloseIcon.svelte";
	import { animDuration } from "$lib/utils/animations";
	import { animateHeight } from "$lib/utils/animateHeight";

	let {
		open = $bindable(),
		title,
		width,
		animateResize = false,
		closeDisabled = false,
		scrollBody = false,
		onclose,
		children,
		footer,
		headerActions,
	} = $props<{
		open: boolean;
		title?: string;
		width?: string;
		animateResize?: boolean;
		closeDisabled?: boolean;
		scrollBody?: boolean;
		onclose?: () => void;
		children?: Snippet;
		footer?: Snippet;
		headerActions?: Snippet;
	}>();

	function close() {
		if (closeDisabled) return;
		open = false;
		onclose?.();
	}

	const fadeDuration = $derived(animDuration(150));
	const flyDuration = $derived(animDuration(250));
	const resizeDuration = $derived(animateResize ? animDuration(300) : 0);
</script>

{#if open}
	<div
		class="modal-overlay"
		onclick={close}
		onkeydown={(e) => e.key === "Escape" && close()}
		role="presentation"
		transition:fade={{ duration: fadeDuration }}
	>
		<div
			class="modal"
			class:animate-resize={animateResize}
			class:scroll-body={scrollBody}
			use:animateHeight={resizeDuration}
			style:--resize-duration={`${resizeDuration}ms`}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="-1"
			style:--modal-default-width={width ?? "400px"}
			transition:fly={{ y: 20, duration: flyDuration }}
		>
			<div class="modal-content">
				<div class="modal-header">
					{#if title}
						<span class="modal-title">{title}</span>
					{/if}
					<div class="modal-header-actions">
						{#if headerActions}{@render headerActions()}{/if}
						<button
							type="button"
							class="action-btn"
							onclick={close}
							disabled={closeDisabled}
							aria-label="Close"
						>
							<CloseIcon size={20} />
						</button>
					</div>
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
		backdrop-filter: blur(var(--backdrop-blur-modal, 4px));
	}

	.modal {
		box-sizing: border-box;
		background: var(--bg-sidebar);
		border: var(--border-width) solid var(--border);
		border-radius: var(--border-radius, 8px);
		width: min(var(--modal-width, var(--modal-default-width)), 90vw);
		max-height: var(--modal-max-height, 90vh);
		overflow-y: auto;
		box-shadow: var(--shadow-lg, 0 20px 40px rgba(0, 0, 0, 0.4));
		transition: width var(--resize-duration)
			cubic-bezier(0.25, 0.8, 0.25, 1);
	}

	.modal.animate-resize {
		scrollbar-gutter: stable;
	}

	.modal-content {
		padding: var(--modal-padding, var(--space-xl));
		display: flex;
		flex-direction: column;
		gap: var(--modal-gap, 20px);
	}

	.modal.scroll-body {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.scroll-body .modal-content {
		min-height: 0;
		max-height: inherit;
		box-sizing: border-box;
	}
	.scroll-body .modal-header,
	.scroll-body .modal-footer {
		flex-shrink: 0;
	}
	.scroll-body .modal-header {
		gap: 8px;
		flex-wrap: wrap;
	}
	.scroll-body .modal-body {
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	@media (prefers-reduced-motion: reduce) {
		.modal {
			transition: none;
		}
	}

	:global(.modal::-webkit-scrollbar) {
		width: var(--scrollbar-size);
	}

	:global(.modal::-webkit-scrollbar-thumb) {
		background: var(--scrollbar-thumb);
		border-radius: var(--scrollbar-radius);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.modal-header-actions {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		margin-left: auto;
	}

	.modal-title {
		font-size: var(--modal-title-size, 1rem);
		font-weight: var(--font-weight-bold);
		letter-spacing: 0.5px;
		color: var(--text-primary);
	}

	.modal-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--modal-footer-gap, 10px);
	}
</style>
