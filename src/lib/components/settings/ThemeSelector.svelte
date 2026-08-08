<script lang="ts">
	import { open, save } from "@tauri-apps/plugin-dialog";
	import { invoke } from "@tauri-apps/api/core";
	import type { ThemeEntry } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { openUrl } from "$lib/api/cubicApi";
	import { SvelteMap } from "svelte/reactivity";
	import CheckIcon from "$lib/icons/CheckIcon.svelte";
	import DownloadIcon from "$lib/icons/DownloadIcon.svelte";
	import Trash from "$lib/icons/Trash.svelte";
	import Icon from "$lib/icons/Icon.svelte";

	let {
		themes = $bindable(),
		value = $bindable(),
		onchange,
		onrefresh,
	}: {
		themes: ThemeEntry[];
		value: string;
		onchange?: (id: string) => void;
		onrefresh?: () => void;
	} = $props();

	let importing = $state(false);

	const groups = $derived.by(() => {
		const map = new SvelteMap<string, ThemeEntry[]>();
		for (const theme of themes) {
			const author =
				theme.author || t("settings.launcher.themesUnknownAuthor");
			let list = map.get(author);
			if (!list) {
				list = [];
				map.set(author, list);
			}
			list.push(theme);
		}
		return map;
	});

	function handleBrowse() {
		openUrl("https://www.cubiclauncher.org/themes");
	}

	async function handleImport() {
		const selected = await open({
			multiple: false,
			filters: [
				{
					name: "Themes",
					extensions: ["zip", "cbth"],
				},
			],
		});
		if (!selected) return;
		importing = true;
		try {
			if (typeof selected === "string") {
				const ext = selected.split(".").pop()?.toLowerCase();
				let entry: ThemeEntry;
				if (ext === "cbth") {
					entry = await invoke<ThemeEntry>("import_theme_cbth", {
						cbthPath: selected,
					});
				} else {
					entry = await invoke<ThemeEntry>("import_theme_zip", {
						zipPath: selected,
					});
				}
				await onrefresh?.();
				selectTheme(`user:${entry.id}`);
			}
		} catch (e) {
			console.error("Error importing theme:", e);
		} finally {
			importing = false;
		}
	}

	async function handleExport(theme: ThemeEntry) {
		const rawId = theme.id.startsWith("user:")
			? theme.id.slice(5)
			: theme.id;
		const savePath = await save({
			defaultPath: `${rawId}.cbth`,
			filters: [{ name: "CBTH Theme", extensions: ["cbth"] }],
		});
		if (!savePath) return;
		try {
			await invoke("export_theme", { id: rawId, dest: savePath });
		} catch (e) {
			console.error("Error exporting theme:", e);
		}
	}

	async function handleDelete(theme: ThemeEntry) {
		const rawId = theme.id.startsWith("user:")
			? theme.id.slice(5)
			: theme.id;
		const confirmed = confirm(
			t("themes.deleteConfirm").replace("{name}", theme.name),
		);
		if (!confirmed) return;
		try {
			await invoke("remove_theme", { id: rawId });
			if (value === theme.id) {
				value = "dark";
				onchange?.("dark");
			}
			onrefresh?.();
		} catch (e) {
			console.error("Error deleting theme:", e);
		}
	}

	function selectTheme(id: string) {
		value = id;
		onchange?.(id);
	}
</script>

<div class="theme-selector">
	<div class="theme-header-actions">
		<button
			type="button"
			class="import-btn"
			onclick={handleImport}
			disabled={importing}
		>
			{importing
				? t("settings.launcher.themesImporting")
				: t("settings.launcher.themesImport")}
		</button>
		<button type="button" class="browse-btn" onclick={handleBrowse}>
			<span>{t("settings.launcher.themesBrowse")}</span>
			<Icon src="/images/icons/instance/external-link.svg" size={14} />
		</button>
	</div>

	{#each [...groups.entries()] as [author, themes] (author)}
		<div class="group">
			<span class="group-title">{author}</span>
			{#each themes as theme (theme.id)}
				<div
					class="theme-row"
					class:active={value === theme.id}
					role="button"
					tabindex="0"
					onclick={() => selectTheme(theme.id)}
					onkeydown={(e) =>
						e.key === "Enter" && selectTheme(theme.id)}
				>
					{#if theme.preview}
						<div class="swatch">
							<div
								class="swatch-bar"
								style="background: {theme.preview.bg}"
							></div>
							<div
								class="swatch-bar"
								style="background: {theme.preview.accent}"
							></div>
							<div
								class="swatch-bar"
								style="background: {theme.preview.text}"
							></div>
						</div>
					{:else}
						<div class="swatch swatch-missing">
							<div
								class="swatch-bar"
								style="background: var(--bg-card)"
							></div>
							<div
								class="swatch-bar"
								style="background: var(--accent)"
							></div>
							<div
								class="swatch-bar"
								style="background: var(--text-primary)"
							></div>
						</div>
					{/if}
					<div class="theme-info">
						<span class="theme-name">{theme.name}</span>
						<span class="theme-meta">
							{theme.author}{#if theme.version}
								v{theme.version}{/if}
						</span>
					</div>
					<div class="theme-actions">
						{#if value === theme.id}
							<div
								class="active-badge"
								title={t("settings.launcher.themesActive")}
							>
								<CheckIcon size={14} />
							</div>
						{/if}
						{#if theme.type !== "builtin"}
							<button
								type="button"
								class="icon-btn"
								title={t("settings.launcher.themesExport")}
								onclick={(e) => {
									e.stopPropagation();
									handleExport(theme);
								}}
							>
								<DownloadIcon size={14} />
							</button>
							<button
								type="button"
								class="icon-btn danger"
								title={t("settings.launcher.themesDelete")}
								onclick={(e) => {
									e.stopPropagation();
									handleDelete(theme);
								}}
							>
								<Trash width={14} height={14} />
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/each}
</div>

<style>
	.theme-selector {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.theme-header-actions {
		display: flex;
		gap: 8px;
	}

	.theme-header-actions .import-btn,
	.theme-header-actions .browse-btn {
		flex: 1;
	}

	.import-btn {
		background: var(--surface-hover);
		border: 1px dashed var(--border);
		color: var(--text-primary);
		padding: 8px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		transition:
			background 0.2s,
			border-color 0.2s;
	}

	.import-btn:hover:not(:disabled) {
		background: var(--surface-active);
		border-color: var(--accent);
		color: var(--accent);
	}

	.import-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.group-title {
		font-size: 0.65rem;
		font-weight: 700;
		letter-spacing: 1.5px;
		text-transform: uppercase;
		color: var(--text-secondary);
		margin-bottom: 4px;
	}

	.theme-row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 8px 10px;
		border: none;
		border-radius: var(--border-radius-sm);
		background: transparent;
		cursor: pointer;
		text-align: left;
		font-family: inherit;
		transition: background 0.15s;
	}

	.theme-row:hover {
		background: var(--surface-hover);
	}

	.theme-row.active {
		background: var(--surface-active);
		outline: 1px solid var(--accent);
	}

	.swatch {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 20px;
		flex-shrink: 0;
	}

	.swatch-bar {
		height: 6px;
		border-radius: 2px;
	}

	.swatch-missing {
		opacity: 0.4;
	}

	.theme-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.theme-name {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.theme-meta {
		font-size: 0.7rem;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.theme-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.active-badge {
		color: var(--accent);
		display: flex;
		align-items: center;
	}

	.icon-btn {
		background: transparent;
		border: none;
		color: var(--text-muted);
		padding: 4px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			color 0.15s,
			background 0.15s;
	}

	.icon-btn:hover {
		color: var(--text-primary);
		background: var(--surface-hover);
	}

	.icon-btn.danger:hover {
		color: var(--color-error);
		background: rgba(var(--color-error-rgb), 0.1);
	}

	.browse-btn {
		background: rgba(var(--accent-rgb), 0.08);
		border: 1px solid rgba(var(--accent-rgb), 0.3);
		color: var(--accent);
		padding: 8px 14px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		transition:
			background 0.2s,
			border-color 0.2s;
	}

	.browse-btn:hover {
		background: rgba(var(--accent-rgb), 0.15);
		border-color: var(--accent);
	}
</style>
