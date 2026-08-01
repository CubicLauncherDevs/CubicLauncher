<script lang="ts">
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		title,
		message,
		onRetry,
	}: {
		title?: string;
		message: string;
		onRetry: () => void;
	} = $props();
</script>

<div class="state-container">
	<div class="icon-wrapper error">
		<Icon src="/images/icons/ui/error.svg" size={32} />
	</div>
	<h3 class="state-title">{title}</h3>
	<p class="state-subtitle error-text">{message}</p>
	<button type="button" class="action-btn retry" onclick={onRetry}>
		{t("common.retry")}
	</button>
</div>

<style>
	.state-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		animation: fadeIn 0.4s ease;
	}

	.state-title {
		font-size: 1.2rem;
		font-weight: 700;
		margin: 0 0 0.5rem 0;
		color: var(--text-primary);
	}

	.state-subtitle {
		font-size: 0.85rem;
		color: var(--text-secondary);
		margin: 0;
		max-width: 80%;
		line-height: 1.5;
	}

	.error-text {
		color: var(--color-error);
		max-width: 90%;
		word-break: break-word;
	}

	.icon-wrapper {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 1.5rem;
		position: relative;
	}

	.icon-wrapper::after {
		content: "";
		position: absolute;
		inset: -4px;
		border-radius: 50%;
		opacity: 0;
		z-index: -1;
		transition: opacity 0.15s;
	}

	.icon-wrapper :global(.icon-svg) {
		width: 32px;
		height: 32px;
	}

	.icon-wrapper.error {
		background: rgba(var(--color-error-rgb), 0.1);
		color: var(--color-error);
		animation: flashPop 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.icon-wrapper.error::after {
		background: radial-gradient(
			circle,
			rgba(var(--color-error-rgb), 0.6) 0%,
			transparent 70%
		);
		box-shadow: var(--glow-error);
		animation: flashGlow 0.7s cubic-bezier(0.16, 1, 0.3, 1) forwards;
	}

	.action-btn.retry {
		margin-top: 1.5rem;
		background: var(--bg-input);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		padding: 0.65rem 2rem;
		border-radius: var(--border-radius-sm);
		font-weight: 600;
		font-size: 0.8rem;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.15s;
	}

	.action-btn.retry:hover {
		background: rgba(var(--surface-rgb), 0.04);
		color: var(--text-primary);
		border-color: var(--text-muted);
	}

	@keyframes flashPop {
		0% {
			opacity: 0;
			transform: scale(0);
		}
		15% {
			opacity: 1;
			transform: scale(1.15);
		}
		35% {
			transform: scale(0.95);
		}
		55% {
			transform: scale(1.03);
		}
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes flashGlow {
		0% {
			opacity: 0;
			transform: scale(0.3);
		}
		15% {
			opacity: 0.9;
			transform: scale(1.3);
		}
		35% {
			opacity: 0.4;
			transform: scale(1);
		}
		100% {
			opacity: 0.3;
			transform: scale(1);
		}
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
