<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { SvelteMap, SvelteSet } from "svelte/reactivity";
	import { save } from "@tauri-apps/plugin-dialog";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";
	import ChevronRightIcon from "$lib/icons/ChevronRightIcon.svelte";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import { showSuccess } from "$lib/state/state.svelte";
	import {
		previewMrpackExport,
		exportInstanceMrpack,
		type MrpackExportEntry,
		type MrpackExportPreview,
	} from "$lib/api/mrpackExportApi";

	let { open = $bindable(), id }: { open: boolean; id: string } = $props();
	let preview = $state.raw<MrpackExportPreview | null>(null);
	let name = $state("");
	let versionId = $state("");
	let summary = $state("");
	let author = $state("");
	let loading = $state(true);
	let working = $state(false);
	let error = $state("");
	let selected = $state<Set<string>>(new Set());
	let disposed = false;

	interface TreeNode {
		entry: MrpackExportEntry;
		label: string;
		children: TreeNode[];
		leaves: string[];
	}

	const files = $derived(
		preview?.entries.filter((entry) => !entry.isDir) ?? [],
	);
	const selectedSize = $derived(
		files.reduce(
			(size, file) => size + (selected.has(file.path) ? file.size : 0),
			0,
		),
	);
	const tree = $derived.by(() => {
		const roots: TreeNode[] = [];
		const nodes = new SvelteMap<string, TreeNode>();
		for (const entry of preview?.entries ?? []) {
			const node: TreeNode = {
				entry,
				label: entry.path.split("/").at(-1) ?? entry.path,
				children: [],
				leaves: entry.isDir ? [] : [entry.path],
			};
			nodes.set(entry.path, node);
			const parent = nodes.get(
				entry.path.slice(0, entry.path.lastIndexOf("/")),
			);
			if (entry.path.includes("/") && parent) parent.children.push(node);
			else roots.push(node);
		}
		for (const node of [...nodes.values()].reverse()) {
			if (node.entry.isDir)
				node.leaves = node.children.flatMap((child) => child.leaves);
		}
		return roots;
	});

	onMount(() => void load());
	onDestroy(() => {
		disposed = true;
	});

	async function load() {
		loading = true;
		error = "";
		try {
			const result = await previewMrpackExport(id);
			if (disposed) return;
			preview = result;
			name = result.name;
			versionId = result.versionId;
			summary = result.summary;
			author = result.author ?? "";
			defaults();
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) loading = false;
		}
	}

	function defaults() {
		selected = new SvelteSet(
			(preview?.entries ?? [])
				.filter((entry) => !entry.isDir && entry.defaultSelected)
				.map((entry) => entry.path),
		);
	}

	function toggle(paths: string[], checked: boolean) {
		const next = new SvelteSet(selected);
		for (const path of paths) {
			if (checked) next.add(path);
			else next.delete(path);
		}
		selected = next;
	}

	function formatSize(bytes: number) {
		const units = ["B", "KiB", "MiB", "GiB", "TiB"];
		const index =
			bytes > 0
				? Math.min(
						Math.floor(Math.log(bytes) / Math.log(1024)),
						units.length - 1,
					)
				: 0;
		return `${(bytes / 1024 ** index).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[index]}`;
	}

	async function exportPack() {
		if (working || loading || !preview || !name.trim() || !versionId.trim())
			return;
		working = true;
		error = "";
		try {
			const filename =
				Array.from(name.trim(), (char) =>
					char.charCodeAt(0) < 32 ? "_" : char,
				)
					.join("")
					.replace(/[<>:"/\\|?*]/g, "_")
					.replace(/[. ]+$/, "") || "modpack";
			const dest = await save({
				defaultPath: `${filename}.mrpack`,
				filters: [{ name: "Modrinth Modpack", extensions: ["mrpack"] }],
			});
			if (!dest || disposed) return;
			const path = await exportInstanceMrpack(id, dest, {
				name: name.trim(),
				versionId: versionId.trim(),
				summary: summary.trim(),
				author: author.trim(),
				selectedPaths: [...selected],
			});
			showSuccess(
				t("notifications.exportTitle"),
				t("notifications.exportSuccess", { path }),
			);
			if (!disposed) open = false;
		} catch (e) {
			if (!disposed) error = String(e);
		} finally {
			if (!disposed) working = false;
		}
	}
</script>

{#snippet checkbox(node: TreeNode)}
	{@const count = node.leaves.filter((path) => selected.has(path)).length}
	<label class="file-label">
		<input
			type="checkbox"
			checked={node.leaves.length > 0 && count === node.leaves.length}
			indeterminate={count > 0 && count < node.leaves.length}
			disabled={working || node.leaves.length === 0}
			onchange={(event) =>
				toggle(node.leaves, event.currentTarget.checked)}
		/>
		<span class="filename" title={node.entry.path}>{node.label}</span>
		<span class="size">{formatSize(node.entry.size)}</span>
	</label>
{/snippet}

{#snippet branch(nodes: TreeNode[], depth: number)}
	<ul>
		{#each nodes as node (node.entry.path)}
			<li>
				{#if node.entry.isDir}
					<details open={depth === 0}>
						<summary>
							<span class="chevron" aria-hidden="true"
								><ChevronRightIcon size={12} /></span
							>
							{@render checkbox(node)}
						</summary>
						{@render branch(node.children, depth + 1)}
					</details>
				{:else}
					<div class="file-row">
						<span class="chevron-spacer" aria-hidden="true"></span>
						{@render checkbox(node)}
					</div>
				{/if}
			</li>
		{/each}
	</ul>
{/snippet}

<ModalBase
	bind:open
	title={t("mrpackExport.title")}
	width="720px"
	closeDisabled={working}
	scrollBody
>
	{#if loading}
		<div class="status" role="status">
			<span class="spinner"></span>
			<span>{t("mrpackExport.loading")}</span>
		</div>
	{:else if preview}
		<fieldset class="panel metadata" disabled={working}>
			<label class="field">
				<span class="input-label">{t("mrpackExport.name")}</span>
				<input
					class="text-input"
					bind:value={name}
					maxlength="512"
					required
				/>
			</label>
			<label class="field">
				<span class="input-label">{t("mrpackExport.version")}</span>
				<input
					class="text-input"
					bind:value={versionId}
					maxlength="512"
					required
				/>
			</label>
			<label class="field">
				<span class="input-label"
					>{t("mrpackExport.authorOptional")}</span
				>
				<input
					class="text-input"
					bind:value={author}
					maxlength="512"
					autocomplete="off"
				/>
			</label>
			<label class="field">
				<span class="input-label">{t("mrpackExport.summary")}</span>
				<textarea
					class="text-input"
					bind:value={summary}
					maxlength="16384"
					rows="2"></textarea>
			</label>
		</fieldset>
		{#if Object.keys(preview.dependencies).length}
			<p class="dependencies">
				<Icon name="instance:puzzle" size={14} />
				<span>
					{Object.entries(preview.dependencies)
						.map(([key, value]) => `${key}: ${value}`)
						.join(" · ")}
				</span>
			</p>
		{/if}
		<section class="panel files">
			<div class="files-head">
				<span class="panel-icon"
					><Icon name="ui:download" size={18} /></span
				>
				<div class="files-heading">
					<strong>{t("mrpackExport.files")}</strong>
					<span class="files-count" aria-live="polite">
						{t("mrpackExport.selection", {
							count: selected.size,
							size: formatSize(selectedSize),
						})}
					</span>
				</div>
				<div class="toolbar">
					<button
						type="button"
						class="btn-secondary"
						disabled={working}
						onclick={defaults}
					>
						<Icon name="ui:refresh" size={14} />
						{t("mrpackExport.defaults")}
					</button>
					<button
						type="button"
						class="btn-secondary"
						disabled={working}
						onclick={() =>
							toggle(
								files.map((file) => file.path),
								true,
							)}
					>
						<Icon name="ui:check" size={14} />
						{t("mrpackExport.selectAll")}
					</button>
					<button
						type="button"
						class="btn-secondary"
						disabled={working}
						onclick={() => (selected = new SvelteSet())}
					>
						<Icon name="ui:close" size={14} />
						{t("mrpackExport.selectNone")}
					</button>
				</div>
			</div>
			<div class="file-tree" aria-label={t("mrpackExport.files")}>
				{@render branch(tree, 0)}
				{#if !files.length}<p class="empty">
						{t("mrpackExport.empty")}
					</p>{/if}
			</div>
			<p class="hint">{t("mrpackExport.hint")}</p>
		</section>
	{/if}
	{#if working}
		<div class="status" role="status">
			<span class="spinner"></span>
			<span>{t("mrpackExport.exporting")}</span>
		</div>
	{/if}
	{#if error}
		<div class="step-error" role="alert">{error}</div>
		{#if !preview && !loading}<button
				type="button"
				class="btn-secondary"
				onclick={load}>{t("mrpackExport.retry")}</button
			>{/if}
	{/if}
	{#snippet footer()}
		<div class="footer-actions">
			<button
				type="button"
				class="btn-secondary"
				disabled={working}
				onclick={() => (open = false)}
			>
				{t("mrpackExport.cancel")}
			</button>
			<button
				type="button"
				class="btn-primary"
				disabled={loading ||
					working ||
					!preview ||
					!name.trim() ||
					!versionId.trim()}
				onclick={exportPack}
			>
				<Icon name="ui:download" size={16} />
				{t("mrpackExport.export")}
			</button>
		</div>
	{/snippet}
</ModalBase>

<style>
	.panel {
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
		background: var(--bg-card);
		padding: 16px;
		margin: 0;
	}
	.metadata {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		align-items: start;
		gap: 14px 16px;
	}
	.metadata .field {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.metadata .input-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 5px;
	}
	.metadata .text-input {
		box-sizing: border-box;
		width: 100%;
		min-width: 0;
		min-height: 36px;
		padding: 8px 12px;
	}
	.metadata textarea.text-input {
		resize: vertical;
	}
	.dependencies {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-secondary);
		font-size: 0.75rem;
		overflow-wrap: anywhere;
	}
	.dependencies :global(.icon-svg) {
		flex-shrink: 0;
		color: var(--text-muted);
	}
	.files {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.files-head {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}
	.panel-icon {
		display: grid;
		place-items: center;
		flex-shrink: 0;
		width: 34px;
		height: 34px;
		border: 1px solid rgba(var(--accent-rgb), 0.15);
		border-radius: var(--border-radius-lg, 8px);
		background: rgba(var(--accent-rgb), 0.07);
		color: var(--accent);
	}
	.files-heading {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		margin-right: auto;
	}
	.files-heading strong {
		color: var(--text-primary);
		font-size: 0.9rem;
		font-weight: 600;
	}
	.files-count,
	.empty {
		color: var(--text-secondary);
		font-size: 0.72rem;
	}
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.toolbar button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.file-tree {
		max-height: 320px;
		overflow: auto;
		scrollbar-gutter: stable;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
		background: var(--bg-input);
		padding: 8px 12px 8px 8px;
	}
	.file-tree ul {
		list-style: none;
		margin: 0;
		padding: 0 0 0 16px;
	}
	.file-tree > ul {
		padding-left: 0;
	}
	summary,
	.file-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	summary {
		list-style: none;
		cursor: pointer;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	.chevron,
	.chevron-spacer {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}
	.chevron {
		color: var(--text-secondary);
		transition: transform var(--transition-fast) ease;
	}
	details[open] > summary .chevron {
		transform: rotate(90deg);
	}
	.file-label {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		min-height: 28px;
		cursor: pointer;
	}
	.file-label input[type="checkbox"] {
		appearance: none;
		-webkit-appearance: none;
		position: relative;
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		margin: 0;
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		transition:
			background var(--transition-fast) ease,
			border-color var(--transition-fast) ease;
	}
	.file-label input[type="checkbox"]:hover:not(:disabled) {
		border-color: var(--text-muted);
	}
	.file-label input[type="checkbox"]:checked,
	.file-label input[type="checkbox"]:indeterminate {
		background: var(--accent);
		border-color: var(--accent);
	}
	.file-label input[type="checkbox"]:checked::after {
		content: "✓";
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		color: var(--accent-text);
		font-size: 10px;
		font-weight: 800;
		line-height: 1;
	}
	.file-label input[type="checkbox"]:indeterminate::after {
		content: "";
		position: absolute;
		top: 50%;
		left: 50%;
		width: 8px;
		height: 2px;
		transform: translate(-50%, -50%);
		background: var(--accent-text);
		border-radius: 1px;
	}
	.file-label input[type="checkbox"]:disabled {
		opacity: var(--disabled-opacity, 0.5);
		cursor: not-allowed;
	}
	.filename {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.size {
		margin-left: auto;
		padding-left: 8px;
		color: var(--text-secondary);
		font-size: 0.85rem;
		white-space: nowrap;
	}
	.hint {
		color: var(--text-secondary);
		font-size: 0.72rem;
		line-height: 1.5;
	}
	.status {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-secondary);
		font-size: var(--font-size-control);
	}
	.spinner {
		width: 14px;
		height: 14px;
		flex-shrink: 0;
		border: 2px solid color-mix(in srgb, currentColor 30%, transparent);
		border-top-color: currentColor;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
		}
	}
	.step-error {
		color: var(--color-error);
		font-size: 0.8rem;
		background: rgba(var(--color-error-rgb), 0.1);
		border: 1px solid rgba(var(--color-error-rgb), 0.2);
		border-radius: 6px;
		padding: 10px;
		text-align: center;
		font-weight: 500;
		overflow-wrap: anywhere;
	}
	.footer-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		width: 100%;
		gap: 10px;
	}
	.footer-actions button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}
</style>
