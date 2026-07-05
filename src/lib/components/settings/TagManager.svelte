<script lang="ts">
	import { launcherStore } from "$lib/state/state.svelte";
	import { createTag, updateTag, deleteTag } from "$lib/api/cubicApi";
	import Trash from "$lib/icons/Trash.svelte";
	import { t } from "$lib/i18n";
	import ModalBase from "$lib/components/layout/ModalBase.svelte";

	let {
		open = $bindable(false),
		onclose,
	}: {
		open: boolean;
		onclose: () => void;
	} = $props();

	let createName = $state("");
	let createColor = $state("");
	let editingId = $state<string | null>(null);
	let editName = $state("");
	let editColor = $state("");
	let saving = $state(false);
	let deleteConfirmId = $state<string | null>(null);
	let showCreate = $state(false);

	const COLORS = ["#ef4444", "#f97316", "#eab308", "#22c55e", "#06b6d4", "#3b82f6", "#8b5cf6", "#ec4899", "#78716c"];

	function resetCreate() {
		createName = "";
		createColor = "";
	}

	async function handleCreate() {
		const name = createName.trim();
		if (!name || saving) return;
		saving = true;
		try {
			await createTag(name, createColor || null);
			resetCreate();
			showCreate = false;
		} finally {
			saving = false;
		}
	}

	function startEdit(tagId: string) {
		const tag = launcherStore.tags.find((t) => t.id === tagId);
		if (!tag) return;
		editingId = tagId;
		editName = tag.name;
		editColor = tag.color ?? "";
		deleteConfirmId = null;
	}

	async function handleSaveEdit() {
		if (!editingId || saving) return;
		saving = true;
		try {
			await updateTag(editingId, editName, editColor || null);
			editingId = null;
		} finally {
			saving = false;
		}
	}

	function cancelEdit() {
		editingId = null;
		editName = "";
		editColor = "";
	}

	function requestDelete(tagId: string) {
		deleteConfirmId = tagId;
		editingId = null;
	}

	async function handleDelete(tagId: string) {
		if (saving) return;
		saving = true;
		try {
			await deleteTag(tagId);
			deleteConfirmId = null;
		} finally {
			saving = false;
		}
	}
</script>

<ModalBase bind:open title={t("tags.manage")} onclose={onclose} width="min(420px, 85vw)">
	<div class="tags-list">
		{#each launcherStore.tags as tag (tag.id)}
			<div class="tag-row" class:confirming={deleteConfirmId === tag.id}>
				{#if deleteConfirmId === tag.id}
					<span class="confirm-text">{t("tags.deleteConfirm")}</span>
					<button
						type="button"
						class="btn danger sm"
						onclick={() => handleDelete(tag.id)}
						disabled={saving}
					>
						{t("tags.delete")}
					</button>
					<button
						type="button"
						class="btn sm"
						onclick={() => (deleteConfirmId = null)}
						disabled={saving}
					>
						{t("common.cancel")}
					</button>
				{:else if editingId === tag.id}
					<div class="edit-wrap">
						<div class="edit-row">
							<input
								type="text"
								bind:value={editName}
								class="tag-input"
								onkeydown={(e) => {
									if (e.key === "Enter") handleSaveEdit();
									if (e.key === "Escape") cancelEdit();
								}}
							/>
							<button
								type="button"
								class="btn primary sm"
								onclick={handleSaveEdit}
								disabled={!editName.trim() || saving}
							>
								{t("common.save")}
							</button>
							<button
								type="button"
								class="btn sm"
								onclick={cancelEdit}
								disabled={saving}
							>
								{t("common.cancel")}
							</button>
						</div>
						<div class="swatch-group">
							{#each COLORS as color}
								<button
									type="button"
									class="swatch sm"
									class:selected={editColor === color}
									style="background: {color}"
									aria-label={color}
									onclick={() => (editColor = editColor === color ? "" : color)}
								></button>
							{/each}
						</div>
					</div>
				{:else}
					<span class="dot" style="background: {tag.color ?? 'var(--text-muted)'}"></span>
					<span class="name">{tag.name}</span>
					<button
						type="button"
						class="btn-icon"
						onclick={() => startEdit(tag.id)}
						title={t("tags.rename")}
					>
						<img src="/images/icons/edit.svg" alt={t("tags.rename")} width="12" height="12" />
					</button>
					<button
						type="button"
						class="btn-icon danger"
						onclick={() => requestDelete(tag.id)}
						title={t("tags.delete")}
					>
						<Trash width="12" height="12" />
					</button>
				{/if}
			</div>
		{/each}
	</div>

	<div class="create-section">
		<button
			type="button"
			class="create-toggle"
			class:expanded={showCreate}
			onclick={() => (showCreate = !showCreate)}
		>
			{t("tags.create")}
			<span class="create-chevron" class:rotated={showCreate}>▸</span>
		</button>
		<div class="create-body" class:expanded={showCreate}>
			<div class="create-inner">
				<div class="create-row">
					<input
						type="text"
						bind:value={createName}
						placeholder={t("tags.namePlaceholder")}
						class="tag-input"
						onkeydown={(e) => e.key === "Enter" && handleCreate()}
					/>
					<button
						type="button"
						class="btn primary"
						onclick={handleCreate}
						disabled={!createName.trim() || saving}
					>
						+
					</button>
				</div>
				<div class="color-row">
					{#each COLORS as color}
						<button
							type="button"
							class="swatch"
							class:selected={createColor === color}
							style="background: {color}"
							aria-label={color}
							onclick={() => (createColor = createColor === color ? "" : color)}
						></button>
					{/each}
				</div>
			</div>
		</div>
	</div>
</ModalBase>

<style>
	.tags-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.tag-row {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 6px;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s;
	}

	.tag-row:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.tag-row.confirming {
		background: rgba(239, 68, 68, 0.06);
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.name {
		flex: 1;
		font-size: 0.85rem;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.confirm-text {
		flex: 1;
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	.tag-input {
		background: var(--bg-input);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 7px 10px;
		border-radius: var(--border-radius-sm);
		font-size: 0.85rem;
		font-family: var(--font-family);
		transition: border-color 0.15s;
	}

	.tag-input:focus {
		outline: none;
		border-color: var(--text-muted);
	}

	.swatch-group {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.edit-wrap {
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 100%;
	}

	.edit-row {
		display: flex;
		gap: 6px;
	}

	.edit-row .tag-input {
		flex: 1;
	}

	.btn-icon {
		background: transparent;
		border: none;
		cursor: pointer;
		padding: 4px;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.4;
	}

	.tag-row:hover .btn-icon {
		opacity: 1;
	}

	.btn-icon:hover {
		background: rgba(255, 255, 255, 0.08);
		opacity: 1;
	}

	.btn-icon.danger {
		color: #ef4444;
	}

	.btn-icon.danger:hover {
		background: rgba(239, 68, 68, 0.12);
	}

	.btn-icon img {
		filter: var(--icon-filter);
		display: block;
	}

	.btn {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 7px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.75rem;
		cursor: pointer;
		font-family: var(--font-family);
		transition: all 0.15s;
		white-space: nowrap;
	}

	.btn:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.primary {
		background: var(--accent, #3b82f6);
		border-color: var(--accent, #3b82f6);
		color: white;
	}

	.btn.primary:hover:not(:disabled) {
		filter: brightness(1.15);
	}

	.btn.sm {
		padding: 4px 10px;
		font-size: 0.7rem;
	}

	.btn.danger {
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.3);
	}

	.btn.danger:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.15);
	}

	.create-section {
		margin-top: 2px;
	}

	.create-toggle {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 4px 6px;
		background: transparent;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		font-family: var(--font-family);
		text-align: left;
		border-radius: var(--border-radius-sm);
		transition: background 0.15s;
	}

	.create-toggle:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.create-chevron {
		font-size: 0.6rem;
		transition: transform 0.2s;
		opacity: 0.4;
		margin-left: auto;
	}

	.create-chevron.rotated {
		transform: rotate(90deg);
	}

	.create-body {
		overflow: hidden;
		max-height: 0;
		transition: max-height 0.2s ease;
	}

	.create-body.expanded {
		max-height: 200px;
	}

	.create-inner {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 6px 6px 2px;
	}

	.create-row {
		display: flex;
		gap: 8px;
	}

	.create-row .tag-input {
		flex: 1;
	}

	.color-row {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.swatch {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		padding: 0;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.swatch:hover {
		transform: scale(1.2);
	}

	.swatch.selected {
		border-color: var(--text-primary);
		box-shadow: 0 0 0 1px var(--bg-sidebar);
	}

	.swatch.sm {
		width: 18px;
		height: 18px;
	}
</style>
