<script lang="ts">
	import { listen } from "@tauri-apps/api/event";
	import { InstState, type InstanceDto } from "$lib/types/types";
	import Loading from "../../icons/Loading.svelte";
	import Check from "../../icons/Check.svelte";
	import { t } from "$lib/i18n";

	let { instance }: { instance: InstanceDto } = $props();

	let lastLog = $state("");
	let lastLevel = $state<"info" | "warn" | "error" | "default">("default");
	let hovered = $state(false);

	const isRunning = $derived(
		instance.status === InstState.Starting || instance.status === InstState.Started,
	);
	const statusLabel = $derived.by(() => {
		switch (instance.status) {
			case InstState.Starting:
				return t("instanceView.status.starting");
			case InstState.Started:
				return t("instanceView.status.started");
			case InstState.Error:
				return t("instanceView.status.error");
			default:
				return "";
		}
	});

	function computeLevel(line: string): "info" | "warn" | "error" | "default" {
		const lower = line.toLowerCase();
		if (lower.includes("[error]") || lower.includes("fatal") || lower.includes("exception") || lower.includes("stacktrace")) {
			return "error";
		}
		if (lower.includes("[warn") || lower.includes("warning")) {
			return "warn";
		}
		if (lower.includes("[info]")) {
			return "info";
		}
		return "default";
	}

	$effect(() => {
		const id = instance.uuid;
		let destroyed = false;

		const unlistenPromise = listen<{ id: string; lines: { line: string; stream: string; timestamp: number }[] }>(
			"instance-log-batch",
			(event) => {
				if (!destroyed && event.payload.id === id && event.payload.lines.length > 0) {
					const last = event.payload.lines[event.payload.lines.length - 1];
					lastLog = last.line;
					lastLevel = last.stream === "stderr"
						? "error"
						: computeLevel(last.line);
				}
			},
		);

		return () => {
			destroyed = true;
			unlistenPromise.then((unlisten) => unlisten?.());
		};
	});

	$effect(() => {
		void instance.uuid;
		lastLog = "";
		lastLevel = "default";
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="status-log-widget"
	class:running={instance.status === InstState.Starting}
	class:started={instance.status === InstState.Started}
	class:error={instance.status === InstState.Error}
	class:hovered
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => (hovered = false)}
>
	<div class="status-log-status">
		{#if instance.status === InstState.Starting}
			<div class="status-log-icon-wrap">
				<Loading class="status-icon-spin" />
			</div>
		{:else if instance.status === InstState.Started}
			<div class="status-log-icon-wrap">
				<Check class="status-icon-static" />
			</div>
		{:else if instance.status === InstState.Error}
			<div class="status-log-icon-wrap">
				<span class="status-error-badge">!</span>
			</div>
		{/if}
		<span class="status-text">{statusLabel}</span>
	</div>
	{#if lastLog}
		<div class="status-log-line">
			<span class="status-log-text {lastLevel}">{lastLog}</span>
		</div>
	{/if}
</div>

<style>
	.status-log-widget {
		position: relative;
		height: 44px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-card);
		overflow: hidden;
		cursor: default;
		transition: background 0.4s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.status-log-widget.running {
		background: var(--color-status-starting);
	}

	.status-log-widget.started {
		background: var(--color-status-started);
	}

	.status-log-widget.error {
		background: rgba(var(--color-error-rgb), 0.15);
	}

	.status-log-status {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		transition:
			transform 0.3s cubic-bezier(0.4, 0, 0.2, 1),
			opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.status-log-widget:hover .status-log-status {
		transform: translateY(-20px);
		opacity: 0;
	}

	.status-log-line {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 14px;
		opacity: 0;
		transform: translateY(16px);
		transition:
			opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1),
			transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		font-size: 0.72rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-log-widget:hover .status-log-line {
		opacity: 1;
		transform: translateY(0);
	}

	.status-log-text {
		font-family: "Cantarell", system-ui, sans-serif;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
		animation: logFadeIn 0.3s ease-out;
	}

	.status-log-text.info {
		color: #b9f6ca;
	}

	.status-log-text.warn {
		color: #fff59d;
	}

	.status-log-text.error {
		color: #ffab91;
	}

	.status-log-text.default {
		color: #e0e0e0;
	}

	.status-log-icon-wrap {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		flex-shrink: 0;
		animation: iconPop 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
	}

	:global(.status-icon-spin) {
		color: white;
		animation: spin 1.2s linear infinite;
		will-change: transform;
		height: 20px;
		width: 20px;
	}

	:global(.status-icon-static) {
		color: white;
		height: 20px;
		width: 20px;
		animation: checkDraw 0.4s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.status-error-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: var(--color-error);
		color: white;
		font-size: 0.65rem;
		font-weight: 800;
		line-height: 1;
		animation: errorPulse 2s ease-in-out infinite;
	}

	.status-text {
		color: white;
		font-size: 0.78rem;
		font-weight: 600;
		text-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(-360deg);
		}
	}

	@keyframes iconPop {
		0% {
			transform: scale(0);
			opacity: 0;
		}
		100% {
			transform: scale(1);
			opacity: 1;
		}
	}

	@keyframes checkDraw {
		0% {
			transform: scale(0.5) rotate(-45deg);
			opacity: 0;
		}
		100% {
			transform: scale(1) rotate(0deg);
			opacity: 1;
		}
	}

	@keyframes errorPulse {
		0%, 100% {
			box-shadow: 0 0 0 0 rgba(var(--color-error-rgb), 0.4);
		}
		50% {
			box-shadow: 0 0 0 6px rgba(var(--color-error-rgb), 0);
		}
	}

	@keyframes logFadeIn {
		0% {
			opacity: 0;
			transform: translateX(-8px);
		}
		100% {
			opacity: 1;
			transform: translateX(0);
		}
	}
</style>
