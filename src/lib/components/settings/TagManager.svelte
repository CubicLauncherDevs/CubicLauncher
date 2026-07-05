<script lang="ts">
	import { launcherStore } from "$lib/state/state.svelte";
	import { createTag, updateTag, deleteTag } from "$lib/api/cubicApi";
	import { t } from "$lib/i18n";

	let {
		open = $bindable(false),
		onclose,
	}: {
		open: boolean;
		onclose: () => void;
	} = $props();

	let editingId = $state<string | null>(null);
	let editName = $state("");
	let editColor = $state("");

	function startEdit(tagId: string) {
		const tag = launcherStore.tags.find((t) => t.id === tagId);
		if (!tag) return;
		editingId = tagId;
		editName = tag.name;
		editColor = tag.color ?? "";
	}

	async function handleSaveEdit() {
		if (!editingId) return;
		await updateTag(editingId, editName, editColor || null);
		editingId = null;
	}

	async function handleDelete(tagId: string) {
		if (!confirm(t("tags.deleteConfirm") || "¿Eliminar esta etiqueta?")) return;
		await deleteTag(tagId);
	}

	async function handleCreate() {
		const name = editName.trim();
		if (!name) return;
		await createTag(name, editColor || null);
		editName = "";
		editColor = "";
	}

	function cancelEdit() {
		editingId = null;
		editName = "";
		editColor = "";
	}

	const COLOR_OPTIONS = ["#ef4444", "#f97316", "#eab308", "#22c55e", "#06b6d4", "#3b82f6", "#8b5cf6", "#ec4899", "#78716c"];
</script>

{#if open}
	<div class="overlay" onclick={onclose} onkeydown={(e) => e.key === "Escape" && onclose()} role="dialog" tabindex="-1">
		<div class="tag-manager" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="document">
			<div class="header">
				<h2>{t("tags.manage")}</h2>
				<button type="button" class="close-btn" onclick={onclose}>✕</button>
			</div>

			<div class="content">
				<div class="create-section">
					<h3>{t("tags.create")}</h3>
					<div class="create-row">
						<input
							type="text"
							bind:value={editName}
							placeholder={t("tags.namePlaceholder")}
							class="tag-input"
						/>
						<div class="color-picker">
							{#each COLOR_OPTIONS as color}
								<button
									type="button"
									class="color-swatch"
									class:selected={editColor === color}
									style="background: {color}"
									onclick={() => (editColor = editColor === color ? "" : color)}
								></button>
							{/each}
							{#if editColor && !COLOR_OPTIONS.includes(editColor)}
								<div class="color-swatch custom" style="background: {editColor}"></div>
							{/if}
						</div>
						<button type="button" class="btn primary" onclick={handleCreate} disabled={!editName.trim()}>
							{t("tags.add")}
						</button>
					</div>
				</div>

				<div class="tags-list">
					{#each launcherStore.tags as tag (tag.id)}
						<div class="tag-row">
							{#if editingId === tag.id}
								<input
									type="text"
									bind:value={editName}
									class="tag-input"
								/>
								<div class="color-picker small">
									{#each COLOR_OPTIONS as color}
										<button
											type="button"
											class="color-swatch"
											class:selected={editColor === color}
											style="background: {color}"
											onclick={() => (editColor = editColor === color ? "" : color)}
										></button>
									{/each}
								</div>
								<button type="button" class="btn primary small" onclick={handleSaveEdit}>{t("common.save")}</button>
								<button type="button" class="btn small" onclick={cancelEdit}>{t("common.cancel")}</button>
							{:else}
								<span class="tag-dot" style="background: {tag.color ?? 'var(--text-muted)'}"></span>
								<span class="tag-name">{tag.name}</span>
								<button type="button" class="btn small" onclick={() => startEdit(tag.id)}>{t("tags.rename")}</button>
								<button type="button" class="btn small danger" onclick={() => handleDelete(tag.id)}>{t("tags.delete")}</button>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.tag-manager {
		background: var(--bg-card);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-md);
		width: 420px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 20px;
		border-bottom: 1px solid var(--border);
	}

	.header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 1.1rem;
		padding: 4px;
	}

	.content {
		padding: 16px 20px;
		overflow-y: auto;
		flex: 1;
	}

	.create-section {
		margin-bottom: 16px;
		padding-bottom: 16px;
		border-bottom: 1px solid var(--border);
	}

	.create-section h3 {
		margin: 0 0 10px;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.create-row {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.color-picker {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
	}

	.color-swatch {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		padding: 0;
		transition: all 0.15s;
	}

	.color-swatch.selected {
		border-color: var(--text-primary);
	}

	.color-swatch:hover {
		transform: scale(1.15);
	}

	.tag-input {
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 8px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		font-family: var(--font-family);
	}

	.tag-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.tags-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.tag-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: var(--border-radius-sm);
		background: rgba(255, 255, 255, 0.02);
	}

	.tag-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tag-name {
		flex: 1;
		font-size: 0.85rem;
	}

	.btn {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 5px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		cursor: pointer;
		font-family: var(--font-family);
		transition: all 0.15s;
	}

	.btn:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.btn.primary {
		background: var(--accent, #3b82f6);
		border-color: var(--accent, #3b82f6);
		color: white;
	}

	.btn.primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.small {
		padding: 3px 8px;
		font-size: 0.7rem;
	}

	.btn.danger {
		color: #ef4444;
	}
</style>
