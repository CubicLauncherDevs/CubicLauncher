<script lang="ts">
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { deleteInstanceFile } from "$lib/api/cubicApi";
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";

	let { instance } = $props<{ instance: InstanceDto }>();
	let screenshots = $state<string[]>([]);
	let selectedImage = $state<string | null>(null);
	let prevInstanceId = $state<string>("");

	async function loadScreenshots() {
		if (instance) {
			screenshots = await invoke<string[]>(
				"get_all_instance_screenshots",
				{
					instanceName: instance.name,
				},
			);
		}
	}

	$effect(() => {
		if (instance.uuid !== prevInstanceId) {
			prevInstanceId = instance.uuid;
			loadScreenshots();
		}
	});

	async function handleDelete(path: string) {
		const filename = path.split(/[\\/]/).pop();
		if (
			filename &&
			confirm(`¿Estás seguro de que deseas eliminar esta captura?`)
		) {
			await deleteInstanceFile(instance.uuid, "screenshots", filename);
			if (selectedImage === path) selectedImage = null;
			await loadScreenshots();
		}
	}
</script>

<div class="screenshots-section">
	<div class="section-header">
		<span class="section-title"
			>{t("instanceView.screenshots.title")} ({screenshots.length})</span
		>
	</div>

	<div class="screenshots-grid">
		{#each screenshots as path (path)}
			<div
				class="screenshot-card"
				role="button"
				tabindex="0"
				onclick={() => (selectedImage = path)}
				onkeydown={(e) => e.key === "Enter" && (selectedImage = path)}
			>
				<img
					src={convertFileSrc(path)}
					alt="Screenshot"
					loading="lazy"
				/>
				<div class="overlay">
					<button
						type="button"
						class="delete-btn"
						onclick={(e) => {
							e.stopPropagation();
							handleDelete(path);
						}}
						title="Eliminar"
					>
						<Icon name="ui:trash" size={18} />
					</button>
				</div>
			</div>
		{/each}

		{#if screenshots.length === 0}
			<div class="empty-state">
				{t("instanceView.screenshots.empty")}
			</div>
		{/if}
	</div>
</div>

{#if selectedImage}
	<div
		class="image-viewer-overlay"
		role="button"
		tabindex="0"
		onclick={() => (selectedImage = null)}
		onkeydown={(e) => e.key === "Escape" && (selectedImage = null)}
	>
		<div
			class="viewer-container"
			role="dialog"
			aria-modal="true"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<img
				src={convertFileSrc(selectedImage)}
				alt="Full size"
				loading="lazy"
			/>
			<button
				type="button"
				class="close-btn"
				onclick={() => (selectedImage = null)}>✕</button
			>
		</div>
	</div>
{/if}

<style>
	.screenshots-section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.screenshots-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 1rem;
	}

	.screenshot-card {
		position: relative;
		aspect-ratio: 16/9;
		background: rgba(var(--surface-rgb), 0.05);
		border-radius: var(--border-radius-sm);
		overflow: hidden;
		cursor: pointer;
		border: 2px solid transparent;
		transition:
			transform 0.2s,
			border-color 0.2s;
	}

	.screenshot-card:hover {
		transform: scale(1.02);
		border-color: var(--accent-primary);
	}

	.screenshot-card img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.overlay {
		position: absolute;
		top: 0;
		right: 0;
		padding: 0.5rem;
		opacity: 0;
		transition: opacity 0.2s;
	}

	.screenshot-card:hover .overlay {
		opacity: 1;
	}

	.delete-btn {
		background: rgba(0, 0, 0, 0.6);
		border: none;
		color: var(--color-error);
		padding: 6px;
		border-radius: var(--border-radius-sm);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.delete-btn:hover {
		background: rgba(var(--color-error-rgb), 0.2);
		color: white;
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 3rem;
		color: var(--text-secondary);
		background: rgba(var(--surface-rgb), 0.02);
		border-radius: var(--border-radius-sm);
		border: 1px dashed rgba(var(--surface-rgb), 0.1);
	}

	.image-viewer-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.9);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		backdrop-filter: blur(var(--backdrop-blur-viewer, 2px));
	}

	.viewer-container {
		position: relative;
		max-width: 90%;
		max-height: 90%;
	}

	.viewer-container img {
		max-width: 100%;
		max-height: 90vh;
		border-radius: var(--border-radius-sm);
		box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
	}

	.close-btn {
		position: absolute;
		top: -40px;
		right: -40px;
		background: transparent;
		border: none;
		color: white;
		font-size: 2rem;
		cursor: pointer;
	}

	@media (max-width: 700px) {
		.screenshots-grid {
			grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		}
	}

	@media (max-width: 550px) {
		.screenshots-grid {
			grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
			gap: 8px;
		}
	}

	@media (max-width: 400px) {
		.screenshots-grid {
			grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
			gap: 6px;
		}
	}
</style>
