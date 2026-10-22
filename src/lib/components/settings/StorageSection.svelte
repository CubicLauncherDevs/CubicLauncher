<script lang="ts">
	import { onDestroy } from "svelte";
	import { ask, open } from "@tauri-apps/plugin-dialog";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import Loading from "$lib/icons/Loading.svelte";
	import {
		getInstancesDirInfo,
		changeInstancesDir,
		resetInstancesDir,
		cancelInstancesDirChange,
		getSharedDirInfo,
		purgeSharedDir,
		changeSharedDir,
		resetSharedDir,
		type InstancesDirInfo,
		type InstancesMoveProgress,
		type SharedDirInfo,
	} from "$lib/api/storageLocation";

	let info = $state<InstancesDirInfo | null>(null);
	let loading = $state(true);
	let moving = $state(false);
	let cancelling = $state(false);
	let error = $state("");
	let progress = $state<InstancesMoveProgress | null>(null);
	let result = $state<{ moved: number; failed: string[] } | null>(null);
	let destroyed = false;

	let sharedInfo = $state<SharedDirInfo | null>(null);
	let sharedLoading = $state(true);
	let sharedBusy = $state(false);
	let sharedError = $state("");
	let sharedResult = $state("");

	let copied = $state<"instances" | "shared" | null>(null);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	const percent = $derived(
		progress && progress.bytes_total > 0
			? Math.min(
					100,
					Math.round(
						(progress.bytes_current / progress.bytes_total) * 100,
					),
				)
			: 0,
	);

	async function refresh() {
		if (!info) loading = true;
		try {
			info = await getInstancesDirInfo();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function refreshShared() {
		if (!sharedInfo) sharedLoading = true;
		try {
			sharedInfo = await getSharedDirInfo();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedLoading = false;
		}
	}

	async function pick() {
		if (moving) return;
		error = "";
		try {
			const path = await open({
				directory: true,
				multiple: false,
				title: t("settings.storage.pickTitle"),
			});
			if (typeof path === "string" && !destroyed) {
				await move(() => changeInstancesDir(path, onProgress));
			}
		} catch (e) {
			error = String(e);
		}
	}

	function reset() {
		if (moving) return;
		error = "";
		void move(() => resetInstancesDir(onProgress));
	}

	function onProgress(value: InstancesMoveProgress) {
		if (!destroyed) progress = value;
	}

	async function move(
		operation: () => Promise<{ moved: number; failed: string[] }>,
	) {
		moving = true;
		progress = null;
		result = null;
		error = "";
		try {
			result = await operation();
			await refresh();
		} catch (e) {
			error = String(e);
		} finally {
			if (!destroyed) {
				moving = false;
				progress = null;
			}
		}
	}

	async function cancel() {
		if (cancelling) return;
		cancelling = true;
		try {
			await cancelInstancesDirChange();
		} finally {
			cancelling = false;
		}
	}

	function formatBytes(bytes: number) {
		const unit = Math.min(
			4,
			Math.floor(Math.log2(Math.max(1, bytes)) / 10),
		);
		return `${(bytes / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KiB", "MiB", "GiB", "TiB"][unit]}`;
	}

	function sharedUsage(usage: { bytes: number; files: number }) {
		if (usage.files === 0) return t("settings.sharedStorage.empty");
		const size = formatBytes(usage.bytes);
		const files = t("settings.sharedStorage.files", {
			count: usage.files,
		});
		return `${size} · ${files}`;
	}

	function categoryLabel(name: string) {
		const key = `settings.sharedStorage.categories.${name}`;
		const label = t(key);
		if (label !== key) return label;
		return name ? name[0].toLocaleUpperCase() + name.slice(1) : name;
	}

	async function copyPath(kind: "instances" | "shared", path: string) {
		try {
			await navigator.clipboard.writeText(path);
			copied = kind;
			clearTimeout(copyTimer);
			copyTimer = setTimeout(() => {
				copied = null;
			}, 1800);
		} catch {
			// Clipboard unavailable; leave the button unchanged.
		}
	}

	function copyInstancesPath() {
		if (info) void copyPath("instances", info.current_dir);
	}

	function copySharedPath() {
		if (sharedInfo) void copyPath("shared", sharedInfo.current_dir);
	}

	async function purgeShared() {
		if (sharedBusy || !sharedInfo) return;
		sharedError = "";
		sharedResult = "";
		try {
			const confirmed = await ask(
				t("settings.sharedStorage.purgeConfirmDesc", {
					size: sharedUsage(sharedInfo.total),
				}),
				{
					title: t("settings.sharedStorage.purgeConfirmTitle"),
					kind: "warning",
					okLabel: t("settings.sharedStorage.purge"),
					cancelLabel: t("common.cancel"),
				},
			);
			if (!confirmed) return;
			sharedBusy = true;
			const usage = await purgeSharedDir();
			sharedResult = t("settings.sharedStorage.purged", {
				size: formatBytes(usage.bytes),
				files: usage.files,
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	async function pickShared() {
		if (sharedBusy) return;
		sharedError = "";
		sharedResult = "";
		try {
			const path = await open({
				directory: true,
				multiple: false,
				title: t("settings.sharedStorage.pickTitle"),
			});
			if (typeof path !== "string" || destroyed) return;
			const confirmed = await ask(
				t("settings.sharedStorage.changeConfirmDesc", {
					size: sharedInfo ? sharedUsage(sharedInfo.total) : "0",
				}),
				{
					title: t("settings.sharedStorage.changeConfirmTitle"),
					kind: "warning",
					okLabel: t("settings.sharedStorage.change"),
					cancelLabel: t("common.cancel"),
				},
			);
			if (!confirmed) return;
			sharedBusy = true;
			const usage = await changeSharedDir(path);
			sharedResult = t("settings.sharedStorage.changed", {
				size: formatBytes(usage.bytes),
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	async function resetShared() {
		if (sharedBusy) return;
		sharedError = "";
		sharedResult = "";
		try {
			sharedBusy = true;
			const usage = await resetSharedDir();
			sharedResult = t("settings.sharedStorage.changed", {
				size: formatBytes(usage.bytes),
			});
			await refreshShared();
		} catch (e) {
			sharedError = String(e);
		} finally {
			sharedBusy = false;
		}
	}

	onDestroy(() => {
		destroyed = true;
		clearTimeout(copyTimer);
	});

	void refresh();
	void refreshShared();
</script>

<div class="storage">
	{#if loading}
		<section class="storage-group" aria-busy="true">
			<header class="group-header">
				<h4 class="group-title">
					<Icon name="instance:folder" size={15} />
					{t("settings.storage.instancesTitle")}
				</h4>
			</header>
			<p class="status-line">
				<Loading size={14} class="state-spinner" />
				{t("common.loading")}
			</p>
		</section>
	{:else if info}
		<section class="storage-group" aria-busy={moving}>
			<header class="group-header">
				<h4 class="group-title">
					<Icon name="instance:folder" size={15} />
					{t("settings.storage.instancesTitle")}
					<span
						class="location-state"
						class:custom={Boolean(info.custom_dir)}
					>
						· {info.custom_dir
							? t("settings.storage.customLocation")
							: t("settings.storage.defaultLocation")}
					</span>
				</h4>
				<p class="hint">{t("settings.storage.description")}</p>
			</header>

			<div class="field">
				<span class="field-label"
					>{t("settings.storage.currentDir")}</span
				>
				<div class="path-line">
					<Icon name="instance:folder" size={13} class="path-icon" />
					<code class="path" title={info.current_dir}
						>{info.current_dir}</code
					>
					<button
						type="button"
						class="copy-btn"
						class:copied={copied === "instances"}
						aria-label={copied === "instances"
							? t("settings.storage.copied")
							: t("settings.storage.copyPath")}
						title={copied === "instances"
							? t("settings.storage.copied")
							: t("settings.storage.copyPath")}
						onclick={copyInstancesPath}
					>
						<Icon
							name={copied === "instances"
								? "ui:check"
								: "ui:copy"}
							size={13}
						/>
						<span
							>{copied === "instances"
								? t("settings.storage.copied")
								: t("settings.storage.copyPath")}</span
						>
					</button>
				</div>
			</div>

			<div class="meta-line">
				<span class="meta-label"
					>{t("settings.storage.instanceCount")}:</span
				>
				<span class="meta-value">{info.instance_count}</span>
			</div>

			{#if moving}
				<div class="progress-block" role="status" aria-live="polite">
					<div class="progress-head">
						<strong>
							{#if progress}
								{t("settings.storage.progress", {
									index: progress.instance_index,
									count: progress.instance_count,
								})}
								· {progress.instance_name}
							{:else}
								{t("settings.storage.preparing")}
							{/if}
						</strong>
						{#if progress && progress.bytes_total > 0}
							<span class="progress-percent">{percent}%</span>
						{/if}
					</div>
					{#if progress}
						<div class="progress-meta">
							<span>
								{progress.strategy === "copy"
									? t("settings.storage.copyingFallback")
									: t("settings.storage.linking")}
							</span>
							{#if progress.bytes_total > 0}
								<span class="progress-bytes">
									{formatBytes(progress.bytes_current)} /
									{formatBytes(progress.bytes_total)}
								</span>
							{/if}
						</div>
						<progress
							class="move-progress"
							max="100"
							value={progress.bytes_total > 0
								? percent
								: undefined}
							aria-label={t("settings.storage.linking")}
						></progress>
						{#if progress.file}
							<small
								class="path progress-file"
								title={progress.file}>{progress.file}</small
							>
						{/if}
					{:else}
						<progress
							class="move-progress"
							aria-label={t("settings.storage.preparing")}
						></progress>
					{/if}
					<div class="progress-actions">
						<button
							type="button"
							class="btn secondary"
							disabled={cancelling}
							onclick={cancel}
						>
							{cancelling
								? t("settings.storage.cancelling")
								: t("common.cancel")}
						</button>
					</div>
				</div>
			{:else if result}
				<div
					class="status-message"
					class:success={result.failed.length === 0}
					class:warning={result.failed.length > 0}
					role="status"
				>
					<Icon
						name={result.failed.length === 0
							? "ui:check-circle"
							: "ui:error"}
						size={15}
						class="status-icon"
					/>
					<div class="status-body">
						{#if result.failed.length === 0}
							<span
								>{t("settings.storage.done", {
									count: result.moved,
								})}</span
							>
						{:else}
							<span>
								{t("settings.storage.partial", {
									failed: result.failed.length,
								})}
							</span>
							<ul class="failure-list">
								{#each result.failed as failure (failure)}
									<li class="path" title={failure}>
										{failure}
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				</div>
			{/if}

			{#if error}
				<div class="status-message error" role="alert">
					<Icon name="ui:error" size={15} class="status-icon" />
					<span class="status-body">{error}</span>
				</div>
			{/if}

			<div class="actions">
				<button
					type="button"
					class="btn primary"
					disabled={moving}
					onclick={pick}
				>
					<Icon name="instance:folder" size={15} />
					{t("settings.storage.change")}
				</button>
				{#if info.custom_dir}
					<button
						type="button"
						class="btn secondary"
						disabled={moving}
						onclick={reset}
					>
						<Icon name="ui:refresh" size={15} />
						{t("settings.storage.reset")}
					</button>
				{/if}
			</div>
		</section>
	{:else if error}
		<p class="status-message error" role="alert">
			<Icon name="ui:error" size={15} class="status-icon" />
			<span class="status-body">{error}</span>
		</p>
	{/if}

	<section class="storage-group" aria-busy={sharedBusy}>
		<header class="group-header">
			<h4 class="group-title">
				<Icon name="instance:database" size={15} />
				{t("settings.sharedStorage.title")}
				{#if sharedInfo}
					<span
						class="location-state"
						class:custom={Boolean(sharedInfo.custom_dir)}
					>
						· {sharedInfo.custom_dir
							? t("settings.storage.customLocation")
							: t("settings.storage.defaultLocation")}
					</span>
				{/if}
			</h4>
			<p class="hint">{t("settings.sharedStorage.description")}</p>
		</header>

		{#if sharedLoading}
			<p class="status-line">
				<Loading size={14} class="state-spinner" />
				{t("common.loading")}
			</p>
		{:else if sharedInfo}
			<div class="field">
				<span class="field-label"
					>{t("settings.sharedStorage.currentDir")}</span
				>
				<div class="path-line">
					<Icon name="instance:folder" size={13} class="path-icon" />
					<code class="path" title={sharedInfo.current_dir}
						>{sharedInfo.current_dir}</code
					>
					<button
						type="button"
						class="copy-btn"
						class:copied={copied === "shared"}
						aria-label={copied === "shared"
							? t("settings.storage.copied")
							: t("settings.storage.copyPath")}
						title={copied === "shared"
							? t("settings.storage.copied")
							: t("settings.storage.copyPath")}
						onclick={copySharedPath}
					>
						<Icon
							name={copied === "shared" ? "ui:check" : "ui:copy"}
							size={13}
						/>
						<span
							>{copied === "shared"
								? t("settings.storage.copied")
								: t("settings.storage.copyPath")}</span
						>
					</button>
				</div>
			</div>

			<div class="meta-line">
				<span class="meta-value"
					>{formatBytes(sharedInfo.total.bytes)}</span
				>
				<span class="meta-label"
					>{t("settings.sharedStorage.usage")}</span
				>
				<span class="meta-separator">·</span>
				<span class="meta-label"
					>{t("settings.sharedStorage.files", {
						count: sharedInfo.total.files,
					})}</span
				>
			</div>

			{#if sharedInfo.breakdown.length > 0}
				<div class="breakdown">
					<span class="field-label"
						>{t("settings.sharedStorage.breakdownTitle")}</span
					>
					<div class="breakdown-list">
						{#each sharedInfo.breakdown as [name, usage] (name)}
							<div class="breakdown-row">
								<div class="breakdown-head">
									<span class="breakdown-name"
										>{categoryLabel(name)}</span
									>
									<span class="breakdown-value">
										{formatBytes(usage.bytes)} ·
										{t("settings.sharedStorage.files", {
											count: usage.files,
										})}
									</span>
								</div>
								<progress
									class="usage-bar"
									max={Math.max(1, sharedInfo.total.bytes)}
									value={usage.bytes}
									aria-label={`${categoryLabel(name)}: ${formatBytes(usage.bytes)}`}
								></progress>
							</div>
						{/each}
					</div>
				</div>
			{:else}
				<p class="empty-hint">
					<Icon name="instance:database" size={13} />
					{t("settings.sharedStorage.empty")}
				</p>
			{/if}

			{#if sharedBusy}
				<p class="status-line" role="status">
					<Loading size={13} class="state-spinner" />
					{t("settings.sharedStorage.working")}
				</p>
			{:else if sharedResult}
				<div class="status-message success" role="status">
					<Icon
						name="ui:check-circle"
						size={15}
						class="status-icon"
					/>
					<span class="status-body">{sharedResult}</span>
				</div>
			{/if}

			{#if sharedError}
				<div class="status-message error" role="alert">
					<Icon name="ui:error" size={15} class="status-icon" />
					<span class="status-body">{sharedError}</span>
				</div>
			{/if}

			<div class="actions">
				<button
					type="button"
					class="btn primary"
					disabled={sharedBusy}
					onclick={pickShared}
				>
					<Icon name="instance:folder" size={15} />
					{t("settings.sharedStorage.change")}
				</button>
				{#if sharedInfo.custom_dir}
					<button
						type="button"
						class="btn secondary"
						disabled={sharedBusy}
						onclick={resetShared}
					>
						<Icon name="ui:refresh" size={15} />
						{t("settings.sharedStorage.reset")}
					</button>
				{/if}
				<button
					type="button"
					class="btn danger"
					disabled={sharedBusy}
					onclick={purgeShared}
				>
					<Icon name="ui:trash" size={15} />
					{t("settings.sharedStorage.purge")}
				</button>
			</div>
		{/if}
	</section>
</div>

<style>
	.storage {
		display: flex;
		flex-direction: column;
		container-type: inline-size;
	}

	.storage-group {
		display: flex;
		flex-direction: column;
		gap: calc(11px * var(--cubic-interface-gap-factor, 1));
		min-width: 0;
		padding: calc(10px * var(--cubic-interface-gap-factor, 1)) 0;
	}

	.storage-group:first-child {
		padding-top: 2px;
	}

	.storage-group:last-child {
		padding-bottom: 0;
	}

	.storage-group + .storage-group {
		border-top: 1px solid var(--border-color);
	}

	.group-header {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}

	.group-title {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 7px;
		min-width: 0;
		margin: 0;
		color: var(--text-primary);
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.location-state {
		color: var(--text-muted);
		font-size: 0.65rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		text-transform: none;
	}

	.location-state.custom {
		color: var(--accent);
	}

	.hint {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--font-size-sm, 0.8rem);
		line-height: 1.5;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}

	.field-label {
		color: var(--text-muted);
		font-size: 0.64rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.path-line {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		padding: 2px 0;
	}

	:global(.path-icon) {
		flex-shrink: 0;
		color: var(--text-muted);
	}

	.path {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-family: var(--font-family-mono);
		font-size: 0.7rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.copy-btn {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex-shrink: 0;
		padding: 3px 5px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: 0.66rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			color 0.15s,
			background 0.15s;
	}

	.copy-btn:hover {
		background: var(--surface-hover);
		color: var(--text-primary);
	}

	.copy-btn.copied {
		color: var(--accent);
	}

	.copy-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.meta-line {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 4px;
		color: var(--text-secondary);
		font-size: 0.75rem;
	}

	.meta-value {
		color: var(--text-primary);
		font-variant-numeric: tabular-nums;
		font-weight: 700;
	}

	.meta-label,
	.meta-separator {
		color: var(--text-muted);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding-top: calc(10px * var(--cubic-interface-gap-factor, 1));
		border-top: 1px solid var(--border-color);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 8px 12px;
		border: 1px solid var(--border-color);
		border-radius: var(--border-radius-sm);
		background: transparent;
		color: var(--text-primary);
		font: inherit;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			color 0.15s;
	}

	.btn:hover:not(:disabled) {
		background: var(--surface-hover);
		border-color: var(--text-muted);
	}

	.btn.primary {
		border-color: rgba(var(--accent-rgb), 0.3);
		color: var(--accent);
	}

	.btn.primary:hover:not(:disabled) {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.08);
	}

	.btn.secondary {
		color: var(--text-secondary);
	}

	.btn.danger {
		margin-left: auto;
		color: var(--text-secondary);
	}

	.btn.danger:hover:not(:disabled) {
		border-color: rgba(var(--color-error-rgb), 0.45);
		background: rgba(var(--color-error-rgb), 0.08);
		color: var(--color-error);
	}

	.btn:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}

	.btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.breakdown {
		display: flex;
		flex-direction: column;
		gap: 9px;
	}

	.breakdown-list {
		display: flex;
		flex-direction: column;
		gap: 9px;
	}

	.breakdown-row {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.breakdown-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
	}

	.breakdown-name {
		color: var(--text-primary);
		font-size: 0.77rem;
		font-weight: 500;
	}

	.breakdown-value {
		color: var(--text-muted);
		font-size: 0.66rem;
		white-space: nowrap;
	}

	.usage-bar,
	.move-progress {
		width: 100%;
		height: 5px;
		overflow: hidden;
		border: none;
		border-radius: var(--border-radius-pill);
		background: var(--bg-input);
		accent-color: var(--accent);
	}

	.usage-bar::-webkit-progress-bar,
	.move-progress::-webkit-progress-bar {
		border-radius: var(--border-radius-pill);
		background: var(--bg-input);
	}

	.usage-bar::-webkit-progress-value,
	.move-progress::-webkit-progress-value {
		border-radius: var(--border-radius-pill);
		background: var(--accent);
	}

	.usage-bar::-moz-progress-bar,
	.move-progress::-moz-progress-bar {
		border-radius: var(--border-radius-pill);
		background: var(--accent);
	}

	.empty-hint {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0;
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.progress-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.progress-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 10px;
		color: var(--text-primary);
		font-size: 0.78rem;
	}

	.progress-head strong {
		font-weight: 600;
		overflow-wrap: anywhere;
	}

	.progress-percent {
		flex-shrink: 0;
		color: var(--accent);
		font-size: 0.72rem;
		font-variant-numeric: tabular-nums;
		font-weight: 700;
	}

	.progress-meta {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 4px 12px;
		color: var(--text-muted);
		font-size: 0.68rem;
	}

	.progress-bytes {
		font-variant-numeric: tabular-nums;
	}

	.progress-file {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.progress-actions {
		display: flex;
		justify-content: flex-end;
	}

	.status-message {
		display: flex;
		align-items: flex-start;
		gap: 7px;
		margin: 0;
		color: var(--text-secondary);
		font-size: 0.75rem;
		line-height: 1.45;
	}

	.status-message.success {
		color: var(--accent);
	}

	.status-message.warning {
		color: var(--color-warning);
	}

	.status-message.error {
		color: var(--color-error);
	}

	:global(.status-icon) {
		flex-shrink: 0;
		margin-top: 1px;
	}

	.status-body {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.failure-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin: 0;
		padding-left: 16px;
	}

	.failure-list .path {
		overflow-wrap: anywhere;
		color: inherit;
		font-size: 0.68rem;
		white-space: normal;
	}

	.status-line {
		display: flex;
		align-items: center;
		gap: 7px;
		margin: 0;
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	:global(.state-spinner) {
		color: var(--text-muted);
		animation: storage-spin 0.8s linear infinite;
	}

	@keyframes storage-spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.state-spinner) {
			animation: none;
		}
	}

	:global(html[data-reduce-motion] .state-spinner) {
		animation: none;
	}

	@container (max-width: 360px) {
		.breakdown-head {
			align-items: flex-start;
			flex-direction: column;
			gap: 2px;
		}

		.copy-btn span {
			display: none;
		}

		.btn.danger {
			margin-left: 0;
		}
	}
</style>
