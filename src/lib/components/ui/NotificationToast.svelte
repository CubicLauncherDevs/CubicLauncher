<script lang="ts">
	import { onDestroy } from "svelte";
	import type { Notification } from "$lib/types/types";
	import { removeNotification } from "$lib/state/state.svelte";

	let dismissTimer: ReturnType<typeof setTimeout> | undefined;
	let completeTimer: ReturnType<typeof setTimeout> | undefined;

	onDestroy(() => {
		clearTimeout(dismissTimer);
		clearTimeout(completeTimer);
	});

	let { notification }: { notification: Notification } = $props();

	const R = 14.1;
	const CIRC = 2 * Math.PI * R;

	let removing = $state(false);
	let isDone = $state(false);
	let iconColor = $derived(typeColor(notification.type));

	const hasProgress = $derived(typeof notification.progress === "number");

	function typeColor(type: string): string {
		const map: Record<string, string> = {
			error: "#c75818",
			warning: "#8a6800",
			success: "#1a7a3c",
			info: "#1a5fa8",
		};
		return map[type] ?? "#444";
	}

	function dismiss() {
		if (removing) return;
		removing = true;
		dismissTimer = setTimeout(
			() => removeNotification(notification.id),
			340,
		);
	}

	function handleComplete() {
		if (isDone) return;
		isDone = true;
		iconColor = "#1a7a3c";
		completeTimer = setTimeout(() => dismiss(), 1400);
	}

	$effect(() => {
		const p = notification.progress;
		if (typeof p === "number" && p >= 100) handleComplete();
	});

	// External progress: reactive offset via CSS transition
	const progressOffset = $derived.by(() => {
		if (!hasProgress) return CIRC;
		return CIRC * (1 - (notification.progress ?? 0) / 100);
	});

	// Countdown via CSS @keyframes (no JS rAF)
	const uid = Math.random().toString(36).slice(2, 8);
	let circleEl = $state<SVGCircleElement | null>(null);

	$effect(() => {
		if (hasProgress) return;
		if (!circleEl || !notification.timeout || notification.timeout <= 0)
			return;

		const style = document.createElement("style");
		const name = `cd-${uid}`;
		style.textContent = `@keyframes ${name} { from { stroke-dashoffset: ${CIRC}; } to { stroke-dashoffset: 0; } }`;
		document.head.appendChild(style);

		circleEl.style.animation = `${name} ${notification.timeout}ms linear forwards`;

		function onEnd() {
			dismiss();
		}
		circleEl.addEventListener("animationend", onEnd, { once: true });

		return () => {
			style.remove();
			circleEl?.removeEventListener("animationend", onEnd);
		};
	});

	const progressSub = $derived.by(() => {
		const p = notification.progress;
		if (typeof p !== "number") return null;
		const total = notification.totalMb;
		if (total)
			return `${Math.round(p)}% · ${Math.round((p / 100) * total)} / ${total} MB`;
		return `${Math.round(p)}%`;
	});
</script>

<div
	class="notification-toast"
	class:removing
	role="button"
	tabindex="0"
	onclick={dismiss}
	onkeydown={(e) => (e.key === "Enter" || e.key === " ") && dismiss()}
>
	<div class="notification-gloss" aria-hidden="true"></div>

	<div class="notification-icon-wrap">
		<svg class="progress-ring" viewBox="0 0 32 32" aria-hidden="true">
			<circle class="track" cx="16" cy="16" r={R} />
			<circle
				class="fill"
				bind:this={circleEl}
				cx="16"
				cy="16"
				r={R}
				stroke={iconColor}
				stroke-dasharray={CIRC}
				stroke-dashoffset={hasProgress ? progressOffset : CIRC}
			/>
		</svg>

		<div class="notification-icon" style:background={iconColor}>
			<div class="notification-gloss-dot" aria-hidden="true"></div>

			{#key isDone ? "done" : notification.type}
				{#if isDone}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="rgba(255,255,255,0.9)"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M2.5 7l3 3 6-6" />
					</svg>
				{:else if notification.type === "error"}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="rgba(255,255,255,0.9)"
						stroke-width="1.6"
						stroke-linecap="round"
						aria-hidden="true"
					>
						<path d="M4 4l6 6M10 4l-6 6" />
					</svg>
				{:else if notification.type === "warning"}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="rgba(255,255,255,0.9)"
						stroke-width="1.6"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path
							d="M7 2.5L1.5 11.5h11L7 2.5zM7 6v2.5M7 10.5v.01"
						/>
					</svg>
				{:else if notification.type === "success"}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="rgba(255,255,255,0.9)"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M2.5 7l3 3 6-6" />
					</svg>
				{:else}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="rgba(255,255,255,0.9)"
						stroke-width="1.6"
						stroke-linecap="round"
						aria-hidden="true"
					>
						<circle cx="7" cy="7" r="5.5" />
						<path d="M7 6.5v3.5M7 4.5v.01" />
					</svg>
				{/if}
			{/key}
		</div>
	</div>

	<div class="notification-body">
		<span class="notification-title">{notification.title}</span>
		{#if notification.message}
			<span class="notification-message">{notification.message}</span>
		{/if}
		{#if progressSub}
			<span class="notification-sub" class:done={isDone}
				>{progressSub}</span
			>
		{/if}
	</div>
</div>

<style>
	.notification-toast {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 16px 9px 9px;
		border-radius: 22px;
		background: rgba(28, 28, 36, 0.84);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-top-color: rgba(255, 255, 255, 0.18);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.07),
			0 4px 24px rgba(0, 0, 0, 0.5);

		position: relative;
		overflow: hidden;
		cursor: pointer;
		user-select: none;
		-webkit-tap-highlight-color: transparent;

		animation: notificationIn 0.3s cubic-bezier(0.2, 0.85, 0.3, 1) both;
		pointer-events: auto;
		will-change: transform, opacity;
	}

	.notification-toast:hover {
		background: rgba(36, 36, 48, 0.9);
	}
	.notification-toast:active {
		transform: scale(0.985);
	}

	.notification-toast.removing {
		animation: notificationOut 0.32s cubic-bezier(0.4, 0, 0.6, 1) forwards;
		pointer-events: none;
	}

	@keyframes notificationIn {
		from {
			opacity: 0;
			transform: translateX(28px) scale(0.96);
		}
		to {
			opacity: 1;
			transform: translateX(0) scale(1);
		}
	}

	@keyframes notificationOut {
		0% {
			opacity: 1;
			transform: translateX(0) scale(1);
		}
		40% {
			opacity: 0;
			transform: translateX(18px) scale(0.97);
		}
		100% {
			opacity: 0;
			transform: translateX(18px) scale(0.97);
		}
	}

	.notification-gloss {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 50%;
		border-radius: 22px 22px 0 0;
		background: linear-gradient(
			180deg,
			rgba(255, 255, 255, 0.05) 0%,
			transparent 100%
		);
		pointer-events: none;
	}

	.notification-icon-wrap {
		position: relative;
		width: 32px;
		height: 32px;
		flex-shrink: 0;
	}

	.progress-ring {
		position: absolute;
		inset: 0;
		transform: rotate(-90deg);
	}

	.progress-ring :global(.track) {
		fill: none;
		stroke: rgba(255, 255, 255, 0.08);
		stroke-width: 1.8;
	}

	.progress-ring :global(.fill) {
		fill: none;
		stroke-width: 1.8;
		stroke-linecap: round;
		transition:
			stroke-dashoffset 0.12s linear,
			stroke 0.4s ease;
	}

	.notification-icon {
		position: absolute;
		inset: 3px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.22),
			0 2px 6px rgba(0, 0, 0, 0.4);
		transition: background 0.4s ease;
		overflow: hidden;
		will-change: transform;
	}

	.notification-icon svg {
		width: 13px;
		height: 13px;
		position: relative;
		z-index: 1;
		flex-shrink: 0;
	}

	.notification-gloss-dot {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 52%;
		border-radius: 50% 50% 0 0 / 50% 50% 0 0;
		background: linear-gradient(
			180deg,
			rgba(255, 255, 255, 0.22) 0%,
			transparent 100%
		);
	}

	.notification-body {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.notification-title {
		font-size: 13px;
		font-weight: 400;
		color: rgba(255, 255, 255, 0.88);
		letter-spacing: 0.01em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.notification-message {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.4);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.notification-sub {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.35);
		font-variant-numeric: tabular-nums;
		letter-spacing: 0.02em;
		transition: color 0.3s;
	}

	.notification-sub.done {
		color: rgba(80, 200, 120, 0.65);
	}
</style>
