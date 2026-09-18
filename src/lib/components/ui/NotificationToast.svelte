<script lang="ts">
	import { onDestroy } from "svelte";
	import type { Notification } from "$lib/types/types";
	import { removeNotification } from "$lib/state/state.svelte";
	import CopyIcon from "$lib/icons/CopyIcon.svelte";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import { animDuration } from "$lib/utils/animations";
	import {
		DEFAULT_NOTIFICATION_PREFERENCES,
		notificationTimeout,
		type NotificationPreferences,
	} from "./notificationPreferences";

	let dismissTimer: ReturnType<typeof setTimeout> | undefined;
	let copyTimer: ReturnType<typeof setTimeout> | undefined;
	let destroyed = false;

	onDestroy(() => {
		clearTimeout(dismissTimer);
		clearTimeout(copyTimer);
		destroyed = true;
	});

	let {
		notification,
		preferences = DEFAULT_NOTIFICATION_PREFERENCES,
	}: {
		notification: Notification;
		preferences?: Readonly<NotificationPreferences>;
	} = $props();

	const R = 14.1;
	const CIRC = 2 * Math.PI * R;

	let removing = $state(false);
	const isDone = $derived(
		typeof notification.progress === "number" &&
			notification.progress >= 100,
	);
	let copied = $state(false);
	const iconColor = $derived(
		isDone ? "var(--color-success)" : typeColor(notification.type),
	);

	const hasProgress = $derived(typeof notification.progress === "number");
	const customized = $derived(preferences.enabled === true);
	const timeout = $derived(
		notificationTimeout(
			notification.timeout,
			preferences.duration_seconds,
			customized,
		),
	);
	const dismissalDelay = $derived(
		hasProgress
			? isDone
				? customized
					? preferences.duration_seconds * 1000
					: 1400
				: undefined
			: timeout,
	);

	const entryDuration = $derived(animDuration(300, 50));
	const exitDuration = $derived(animDuration(340, 50));

	function typeColor(type: string): string {
		const map: Record<string, string> = {
			error: "var(--color-error)",
			warning: "var(--color-warning)",
			success: "var(--color-success)",
			info: "var(--color-info)",
		};
		return map[type] ?? "var(--text-muted)";
	}

	function dismiss() {
		if (removing) return;
		removing = true;
		dismissTimer = setTimeout(
			() => removeNotification(notification.id),
			exitDuration + 20,
		);
	}

	async function copyMessage(event: MouseEvent) {
		event.stopPropagation();
		if (!notification.message || copied) return;
		try {
			await navigator.clipboard.writeText(notification.message);
		} catch {
			return;
		}
		if (destroyed) return;
		copied = true;
		clearTimeout(copyTimer);
		copyTimer = setTimeout(() => {
			copied = false;
		}, 1500);
	}

	// External progress: reactive offset via CSS transition
	const progressOffset = $derived.by(() => {
		if (!hasProgress) return CIRC;
		return CIRC * (1 - (notification.progress ?? 0) / 100);
	});

	// One shared CSS animation and one timeout per timed toast. Progress updates
	// do not restart dismissal; persistent notices never acquire a timer.
	$effect(() => {
		if (removing || !dismissalDelay || dismissalDelay <= 0) return;
		const timer = setTimeout(dismiss, dismissalDelay);
		return () => clearTimeout(timer);
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
	class:customized
	style="--notification-in-duration: {entryDuration}ms; --notification-out-duration: {exitDuration}ms; --notification-timeout: {timeout ??
		0}ms; --notification-circumference: {CIRC};"
	role="button"
	tabindex="0"
	onclick={dismiss}
	onkeydown={(e) => {
		if (
			e.target === e.currentTarget &&
			(e.key === "Enter" || e.key === " ")
		) {
			e.preventDefault();
			dismiss();
		}
	}}
>
	<div class="notification-gloss" aria-hidden="true"></div>

	<div class="notification-icon-wrap">
		<svg
			class="progress-ring preserve-motion"
			viewBox="0 0 32 32"
			aria-hidden="true"
		>
			<circle class="track" cx="16" cy="16" r={R} />
			{#key timeout}
				<circle
					class="fill"
					class:countdown={!hasProgress && !!timeout && timeout > 0}
					cx="16"
					cy="16"
					r={R}
					style:stroke={iconColor}
					stroke-dasharray={CIRC}
					stroke-dashoffset={hasProgress ? progressOffset : CIRC}
				/>
			{/key}
		</svg>

		<div class="notification-icon" style:background={iconColor}>
			<div class="notification-gloss-dot" aria-hidden="true"></div>

			{#key isDone ? "done" : notification.type}
				{#if isDone}
					<svg
						viewBox="0 0 14 14"
						fill="none"
						stroke="var(--color-on-success)"
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
						stroke="var(--color-on-error)"
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
						stroke="var(--color-on-warning)"
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
						stroke="var(--color-on-success)"
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
						stroke="var(--color-on-info)"
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

	{#if notification.type === "error"}
		<button
			type="button"
			class="notification-copy"
			class:copied
			aria-label="Copiar mensaje"
			onclick={copyMessage}
		>
			{#if copied}
				<CheckIcon size={14} color="var(--color-success)" />
			{:else}
				<CopyIcon size={14} />
			{/if}
		</button>
	{/if}
</div>

<style>
	.notification-toast {
		display: flex;
		align-items: center;
		gap: var(--toast-gap, 10px);
		padding: var(
			--notification-padding,
			var(--toast-padding, 9px 16px 9px 9px)
		);
		border-radius: var(--toast-radius, 22px);
		background: var(--toast-bg);
		border: 1px solid var(--toast-border);
		border-top-color: rgba(var(--surface-rgb), 0.18);
		box-shadow: var(--toast-shadow, var(--shadow-lg));

		position: relative;
		overflow: hidden;
		cursor: pointer;
		user-select: none;
		-webkit-tap-highlight-color: transparent;

		animation: notificationIn var(--notification-in-duration, 0.3s)
			cubic-bezier(0.2, 0.85, 0.3, 1) both;
		pointer-events: auto;
		will-change: transform, opacity;
		box-sizing: border-box;
		max-width: 100%;
		width: 100%;
		flex-shrink: 0;
	}

	.notification-toast:hover {
		background: var(--surface-hover);
	}
	.notification-toast:active {
		transform: scale(0.985);
	}

	.notification-toast:focus-visible,
	.notification-copy:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -3px;
	}

	.customized .notification-icon-wrap {
		align-self: flex-start;
	}

	.customized .notification-copy {
		align-self: flex-start;
		margin-top: 4px;
	}

	.notification-toast.removing {
		animation: notificationOut var(--notification-out-duration, 0.32s)
			cubic-bezier(0.4, 0, 0.6, 1) forwards;
		pointer-events: none;
	}

	@keyframes notificationIn {
		from {
			opacity: 0;
			transform: translate(
					var(--notification-offset-x, 28px),
					var(--notification-offset-y, 0px)
				)
				scale(0.96);
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
		40%,
		100% {
			opacity: 0;
			transform: translate(
					var(--notification-offset-x, 18px),
					var(--notification-offset-y, 0px)
				)
				scale(0.97);
		}
	}

	.notification-gloss {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 50%;
		border-radius: var(--toast-radius, 22px) var(--toast-radius, 22px) 0 0;
		background: linear-gradient(
			180deg,
			var(--surface-card) 0%,
			transparent 100%
		);
		pointer-events: none;
	}

	.notification-icon-wrap {
		position: relative;
		width: var(--notification-icon-size, 32px);
		height: var(--notification-icon-size, 32px);
		flex-shrink: 0;
	}

	.progress-ring {
		position: absolute;
		inset: 0;
		transform: rotate(-90deg);
	}

	.progress-ring :global(.track) {
		fill: none;
		stroke: var(--surface-hover);
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

	.fill.countdown {
		animation: notificationCountdown var(--notification-timeout) linear
			forwards;
		/* This duration represents time, not a decorative motion preference. */
		animation-duration: var(--notification-timeout) !important;
	}

	@keyframes notificationCountdown {
		from {
			stroke-dashoffset: var(--notification-circumference);
		}
		to {
			stroke-dashoffset: 0;
		}
	}

	.notification-icon {
		position: absolute;
		inset: 3px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow: var(--toast-icon-shadow, var(--shadow-sm));
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
			rgba(var(--surface-rgb), 0.22) 0%,
			transparent 100%
		);
	}

	.notification-body {
		display: flex;
		flex-direction: column;
		gap: var(--notification-body-gap, 2px);
		flex: 1;
		min-width: 0;
	}

	.notification-title {
		font-size: var(
			--notification-title-size,
			var(--toast-title-size, 0.9286rem)
		);
		font-weight: var(--notification-title-weight, 400);
		text-transform: var(--notification-title-case, none);
		color: var(--text-primary);
		letter-spacing: 0.01em;
		white-space: normal;
		overflow-wrap: anywhere;
		line-height: 1.35;
	}

	.notification-message {
		font-size: var(--notification-message-size, 0.7857rem);
		color: var(--text-secondary);
		text-transform: none;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		line-height: 1.45;
		max-height: min(14rem, 40dvh);
		overflow-y: auto;
	}

	.notification-sub {
		font-size: var(--notification-message-size, 0.7857rem);
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
		letter-spacing: 0.02em;
		transition: color 0.3s;
	}

	.notification-sub.done {
		color: var(--color-success);
	}

	.notification-copy {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		padding: 0;
		margin: 0;
		border: none;
		border-radius: 50%;
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		opacity: 0;
		transition:
			opacity 0.2s ease,
			background 0.2s ease,
			color 0.2s ease;
		pointer-events: auto;
		flex-shrink: 0;
	}

	.notification-copy:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.notification-copy:active {
		transform: scale(0.92);
	}

	.notification-copy.copied {
		color: var(--color-success);
	}

	/* The uncustomized mode retains the original compact, content-sized toasts. */
	.notification-toast:not(.customized) {
		width: auto;
		align-items: center;
		padding: var(--toast-padding, 9px 16px 9px 9px);
	}
	.notification-toast:not(.customized) .notification-body {
		flex: 0 1 auto;
		gap: 2px;
	}
	.notification-toast:not(.customized) .notification-title {
		font-size: var(--toast-title-size, 0.9286rem);
		font-weight: 400;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		line-height: inherit;
	}
	.notification-toast:not(.customized) .notification-message {
		font-size: 11px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-height: none;
		line-height: inherit;
	}
	.notification-toast:not(.customized) .notification-sub {
		font-size: 11px;
	}
	.notification-toast:hover .notification-copy,
	.notification-toast:focus-within .notification-copy,
	.notification-copy.copied {
		opacity: 1;
	}

	@media (hover: none) {
		.notification-copy {
			opacity: 1;
		}
	}
</style>
