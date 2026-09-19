<script lang="ts">
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import MarkdownRenderer from "./MarkdownRenderer.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import { updater } from "$lib/api/updaterServices";
	import { updaterState } from "$lib/state/updaterState.svelte";

	const statusTitle = $derived(t(`updater.status.${updaterState.status}`));
	const busy = $derived(
		["checking", "downloading", "installing", "restarting"].includes(
			updaterState.status,
		),
	);
	const subtitle = $derived(
		updaterState.status === "error"
			? t(`updater.errors.${updaterState.failedOperation}`)
			: t(`updater.messages.${updaterState.status}`),
	);

	function formatBytes(bytes: number): string {
		return bytes >= 1024 * 1024
			? `${(bytes / (1024 * 1024)).toFixed(1)} MB`
			: `${Math.round(bytes / 1024)} KB`;
	}

	function manageFocus(node: HTMLElement) {
		const dialog = node.closest<HTMLElement>('[role="dialog"]');
		if (!dialog) return;
		const previous = document.activeElement;
		dialog.setAttribute("aria-labelledby", "update-modal-title");
		queueMicrotask(() => {
			if (dialog.isConnected) dialog.focus({ preventScroll: true });
		});
		function handleKey(event: KeyboardEvent) {
			if (event.key === "Escape") {
				event.preventDefault();
				event.stopPropagation();
				updater.close();
			} else if (event.key === "Tab" && dialog) {
				const controls = Array.from(
					dialog.querySelectorAll<HTMLElement>(
						'button:not(:disabled), a[href], summary, [tabindex="0"]',
					),
				).filter((element) => element.getClientRects().length > 0);
				const first = controls[0];
				const last = controls.at(-1);
				if (
					event.shiftKey &&
					(document.activeElement === first ||
						document.activeElement === dialog)
				) {
					event.preventDefault();
					last?.focus();
				} else if (
					!event.shiftKey &&
					(document.activeElement === last ||
						document.activeElement === dialog)
				) {
					event.preventDefault();
					first?.focus();
				}
			}
		}
		dialog.addEventListener("keydown", handleKey, true);
		return {
			destroy() {
				dialog.removeEventListener("keydown", handleKey, true);
				if (previous instanceof HTMLElement && previous.isConnected)
					previous.focus({ preventScroll: true });
			},
		};
	}
</script>

<ModalBase
	bind:open={updaterState.open}
	title={t("updater.title")}
	width="560px"
	onclose={updater.close}
>
	<div class="update-content" use:manageFocus>
		<div class="update-heading" aria-live="polite" aria-atomic="true">
			<div
				class="status-icon"
				class:error={updaterState.status === "error"}
			>
				{#if busy}
					<Loading class="update-spinner" />
				{:else}
					<Icon
						name={updaterState.status === "error"
							? "ui:error"
							: updaterState.status === "updated" ||
								  updaterState.status === "ready"
								? "ui:check-circle"
								: "ui:download"}
						size={26}
					/>
				{/if}
			</div>
			<div>
				<h2 id="update-modal-title">{statusTitle}</h2>
				<p>{subtitle}</p>
			</div>
		</div>

		<div class="versions">
			<div>
				<span>{t("settings.launcher.currentVersion")}</span><strong
					>v{__APP_VERSION__}</strong
				>
			</div>
			{#if updaterState.update}
				<Icon name="ui:chevron-right" size={16} />
				<div>
					<span>{t("settings.launcher.available")}</span><strong
						class="new-version"
						>v{updaterState.update.version}</strong
					>
				</div>
			{/if}
		</div>

		{#if updaterState.status === "downloading"}
			<div class="download-progress">
				<progress
					aria-label={t("updater.status.downloading")}
					max="100"
					value={updaterState.progress ?? undefined}
				></progress>
				<div class="progress-labels">
					<span
						>{formatBytes(
							updaterState.downloadedBytes,
						)}{#if updaterState.totalBytes !== null}
							/ {formatBytes(updaterState.totalBytes)}{/if}</span
					>
					<span
						>{updaterState.progress === null
							? t("updater.receiving")
							: `${updaterState.progress}%`}</span
					>
				</div>
			</div>
		{/if}

		{#if updaterState.error}
			<p class="error-details" role="alert">{updaterState.error}</p>
		{/if}

		{#if updaterState.update?.body}
			<details class="release-notes" open>
				<summary>{t("updater.releaseNotes")}</summary>
				<div class="notes-content">
					<MarkdownRenderer
						source={updaterState.update.body}
						onLinkClick={openUrl}
					/>
				</div>
			</details>
		{/if}
	</div>

	{#snippet footer()}
		<div class="update-actions">
			<button type="button" class="secondary" onclick={updater.close}>
				{updaterState.status === "available"
					? t("updater.notNow")
					: updaterState.status === "ready"
						? t("updater.later")
						: t("updater.close")}
			</button>
			{#if updaterState.status === "available"}
				<button
					type="button"
					class="primary"
					onclick={() => updater.download()}
					><Icon name="ui:download" size={16} />{t(
						"updater.download",
					)}</button
				>
			{:else if updaterState.status === "ready"}
				<button
					type="button"
					class="primary"
					onclick={() => updater.install()}
					><Icon name="ui:refresh" size={16} />{t(
						"updater.install",
					)}</button
				>
			{:else if updaterState.status === "error"}
				<button
					type="button"
					class="primary"
					onclick={() => updater.retry()}
					><Icon name="ui:refresh" size={16} />{t(
						"updater.retry",
					)}</button
				>
			{/if}
		</div>
	{/snippet}
</ModalBase>

<style>
	.update-content {
		display: flex;
		flex-direction: column;
		gap: 20px;
		min-width: 0;
	}
	.update-heading {
		display: flex;
		align-items: flex-start;
		gap: 14px;
	}
	.status-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		flex-shrink: 0;
		border-radius: var(--border-radius);
		background: var(--surface-selected);
		color: var(--accent);
	}
	.status-icon.error {
		color: var(--color-error);
	}
	:global(.update-spinner) {
		width: 24px;
		height: 24px;
	}
	h2 {
		margin: 0 0 6px;
		font-size: 1.1rem;
		color: var(--text-primary);
	}
	p {
		margin: 0;
		font-size: 0.82rem;
		line-height: 1.6;
		color: var(--text-secondary);
	}
	.versions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 18px;
		padding: 14px;
		background: var(--surface-input);
		border-radius: var(--border-radius-sm);
	}
	.versions > div {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}
	.versions span {
		font-size: 0.72rem;
		color: var(--text-secondary);
	}
	.versions strong {
		font-size: 0.9rem;
		overflow-wrap: anywhere;
		color: var(--text-primary);
	}
	.versions .new-version {
		color: var(--accent);
	}
	.download-progress {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	progress {
		width: 100%;
		height: 8px;
		accent-color: var(--accent);
	}
	.progress-labels {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 6px;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		color: var(--text-secondary);
	}
	.error-details {
		padding: 12px;
		background: var(--surface-input);
		border: 1px solid var(--color-error);
		border-radius: var(--border-radius-sm);
		color: var(--color-error);
		overflow-wrap: anywhere;
		max-height: 140px;
		overflow-y: auto;
	}
	.release-notes {
		border-top: 1px solid var(--border);
		padding-top: 14px;
	}
	summary {
		cursor: pointer;
		color: var(--text-primary);
		font-size: 0.82rem;
		font-weight: 600;
	}
	.notes-content {
		margin-top: 12px;
		max-height: 220px;
		overflow-y: auto;
		padding-right: 8px;
	}
	.update-actions {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 10px;
		width: 100%;
	}
	button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		font: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.secondary {
		background: transparent;
		color: var(--text-secondary);
	}
	.secondary:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}
	.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
	}
	.primary:hover {
		background: var(--accent-hover);
	}
	button:focus-visible,
	summary:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}
</style>
