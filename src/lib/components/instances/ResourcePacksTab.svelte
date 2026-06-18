<script lang="ts">
	import {
		getInstanceResourcePacks,
		deleteInstanceFile,
		addInstanceFile,
	} from "$lib/api/cubicApi";
	import { type ModDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import { open } from "@tauri-apps/plugin-dialog";

	let { instanceId } = $props<{ instanceId: string }>();
	let packs = $state<ModDto[]>([]);
	let isLoading = $state(false);
	let prevInstanceId = $state<string>("");

	async function loadPacks() {
		if (instanceId) {
			isLoading = true;
			packs = await getInstanceResourcePacks(instanceId);
			isLoading = false;
		}
	}

	$effect(() => {
		if (instanceId !== prevInstanceId) {
			prevInstanceId = instanceId;
			loadPacks();
		}
	});

	async function handleDelete(pack: ModDto) {
		if (confirm(`¿Estás seguro de que deseas eliminar ${pack.filename}?`)) {
			await deleteInstanceFile(
				instanceId,
				"resourcepacks",
				pack.filename,
			);
			await loadPacks();
		}
	}

	async function handleAdd() {
		const selected = await open({
			multiple: true,
			filters: [
				{
					name: "Resource Pack",
					extensions: ["zip"],
				},
			],
		});

		if (selected && Array.isArray(selected)) {
			for (const path of selected) {
				await addInstanceFile(instanceId, "resourcepacks", path);
			}
			await loadPacks();
		} else if (selected && typeof selected === "string") {
			await addInstanceFile(instanceId, "resourcepacks", selected);
			await loadPacks();
		}
	}
</script>

<div class="resources-section">
	<div class="section-header">
		<span class="section-title"
			>{t("instanceView.resources.title")} ({packs.length})</span
		>
		<button type="button" class="add-btn" onclick={handleAdd}>
			{t("instanceView.resources.addBtn")}
		</button>
	</div>

	<div class="packs-grid">
		{#each packs as pack (pack.filename)}
			<div class="pack-card">
				<div class="pack-icon">
					{#if pack.icon}
						<img src={pack.icon} alt={pack.name} />
					{:else}
						<div class="mod-icon-placeholder">🎨</div>
					{/if}
				</div>
				<div class="pack-info">
					<span class="pack-name" title={pack.name}>{pack.name}</span>
					<p class="pack-description" title={pack.description}>
						{pack.description ||
							t("instanceView.mods.noDescription")}
					</p>
				</div>
				<button
					type="button"
					class="delete-btn"
					onclick={() => handleDelete(pack)}
					title="Eliminar"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						><path d="M3 6h18" /><path
							d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"
						/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" /><line
							x1="10"
							y1="11"
							x2="10"
							y2="17"
						/><line x1="14" y1="11" x2="14" y2="17" /></svg
					>
				</button>
			</div>
		{/each}

		{#if packs.length === 0 && !isLoading}
			<div class="empty-state">
				{t("instanceView.resources.empty")}
			</div>
		{/if}
	</div>
</div>

<style>
	.resources-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.add-btn {
		padding: 0.5rem 1rem;
		background: var(--accent-primary);
		color: white;
		border: none;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		font-weight: 500;
		transition: opacity 0.2s;
	}

	.add-btn:hover {
		opacity: 0.9;
	}

	.packs-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 0.75rem;
	}

	.pack-card {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		padding: 14px;
		display: flex;
		gap: 14px;
		align-items: flex-start;
		transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
	}

	.pack-card:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.pack-icon {
		width: 48px;
		height: 48px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		overflow: hidden;
	}

	.pack-icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.pack-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.pack-name {
		font-size: 0.88rem;
		font-weight: 700;
		color: var(--text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-description {
		font-size: 0.72rem;
		color: var(--text-secondary);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		line-height: 1.4;
		margin-top: 4px;
	}

	.delete-btn {
		background: transparent;
		border: none;
		color: #ff4444;
		cursor: pointer;
		padding: 4px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.2s;
	}

	.delete-btn:hover {
		background: rgba(255, 68, 68, 0.1);
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 2rem;
		color: var(--text-secondary);
		background: rgba(255, 255, 255, 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}

	@media (max-width: 700px) {
		.packs-grid {
			grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		}
	}

	@media (max-width: 550px) {
		.packs-grid {
			grid-template-columns: 1fr;
			gap: 8px;
		}
	}

	@media (max-width: 400px) {
		.packs-grid {
			grid-template-columns: 1fr;
			gap: 6px;
		}
	}
</style>
