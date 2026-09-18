<script lang="ts">
	import { onMount, onDestroy, untrack, tick } from "svelte";
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { deleteInstanceFile } from "$lib/api/cubicApi";
	import type { InstanceDto } from "$lib/types/types";
	import { t } from "$lib/i18n";
	import Icon from "$lib/icons/Icon.svelte";
	import { createScreenshotThumbnails } from "$lib/state/screenshotThumbnails";
	import { observeThemeMetrics } from "$lib/utils/themeMetrics";

	let { instance } = $props<{ instance: InstanceDto }>();
	let screenshots = $state<string[]>([]);
	let selectedImage = $state<string | null>(null);
	let loading = $state(true);
	let error = $state("");
	let deleting = $state<string | null>(null);
	let grid: HTMLDivElement;
	let width = $state(0);
	let height = $state(0);
	let scrollTop = $state(0);
	let visible = $state(true);
	let metrics = $state({ card: 200, gap: 16 });
	let frame: number | undefined;
	let alive = true;
	let request = 0;
	let currentInstanceId = "";
	const thumbnails = createScreenshotThumbnails((path) =>
		invoke<string | null>("get_screenshot_thumbnail", {
			instanceId: currentInstanceId,
			filename: path.split(/[\\/]/).pop(),
		}),
	);
	const columns = $derived(
		Math.max(
			1,
			Math.floor((width + metrics.gap) / (metrics.card + metrics.gap)),
		),
	);
	const rowHeight = $derived(
		Math.max(
			1,
			(((width - metrics.gap * (columns - 1)) / columns) * 9) / 16 +
				metrics.gap,
		),
	);
	const rowCount = $derived(Math.ceil(screenshots.length / columns));
	const firstRow = $derived(
		Math.max(
			0,
			Math.min(rowCount - 1, Math.floor(scrollTop / rowHeight)) - 2,
		),
	);
	const lastRow = $derived(
		Math.min(rowCount, Math.ceil((scrollTop + height) / rowHeight) + 2),
	);
	const rows = $derived(
		Array.from(
			{ length: Math.max(0, lastRow - firstRow) },
			(_, index) => firstRow + index,
		),
	);

	async function loadScreenshots(id = instance.uuid, name = instance.name) {
		const token = ++request;
		loading = true;
		error = "";
		thumbnails.reset();
		try {
			const result = await invoke<string[]>(
				"get_all_instance_screenshots",
				{ instanceName: name },
			);
			if (!alive || token !== request || id !== currentInstanceId) return;
			screenshots = result;
		} catch (err) {
			if (alive && token === request) error = String(err);
		} finally {
			if (alive && token === request) loading = false;
		}
	}

	$effect(() => {
		const id = instance.uuid;
		const name = instance.name;
		untrack(() => {
			currentInstanceId = id;
			screenshots = [];
			selectedImage = null;
			deleting = null;
			scrollTop = 0;
			if (grid) grid.scrollTop = 0;
			void loadScreenshots(id, name);
		});
	});

	$effect(() => {
		const paths =
			!loading && visible && width > 0 && height > 0
				? screenshots.slice(firstRow * columns, lastRow * columns)
				: [];
		untrack(() => thumbnails.request(paths));
	});

	function handleScroll() {
		if (frame !== undefined) return;
		frame = requestAnimationFrame(() => {
			frame = undefined;
			scrollTop = grid.scrollTop;
		});
	}

	onMount(() => {
		const stopMetrics = observeThemeMetrics(
			grid,
			{
				card: { variable: "--screenshot-min-width", fallback: 200 },
				gap: {
					variable: "--screenshot-gap",
					fallback: 16,
					allowZero: true,
				},
			},
			(values) => {
				metrics = values;
			},
		);
		const observer = new ResizeObserver(([entry]) => {
			width = entry.contentRect.width;
			height = entry.contentRect.height;
			scrollTop = grid.scrollTop;
		});
		observer.observe(grid);
		const visibility = () => {
			visible = !document.hidden;
		};
		visibility();
		document.addEventListener("visibilitychange", visibility);
		return () => {
			observer.disconnect();
			stopMetrics();
			document.removeEventListener("visibilitychange", visibility);
		};
	});
	onDestroy(() => {
		alive = false;
		request++;
		thumbnails.destroy();
		if (frame !== undefined) cancelAnimationFrame(frame);
	});

	async function navigate(event: KeyboardEvent, index: number) {
		const targets: Record<string, number> = {
			ArrowLeft: index - 1,
			ArrowRight: index + 1,
			ArrowUp: index - columns,
			ArrowDown: index + columns,
			Home: 0,
			End: screenshots.length - 1,
		};
		if (!(event.key in targets)) return;
		event.preventDefault();
		const target = Math.max(
			0,
			Math.min(screenshots.length - 1, targets[event.key]),
		);
		const top = Math.floor(target / columns) * rowHeight;
		if (top < grid.scrollTop) grid.scrollTop = top;
		else if (top + rowHeight > grid.scrollTop + height)
			grid.scrollTop = top + rowHeight - height;
		scrollTop = grid.scrollTop;
		await tick();
		if (alive)
			grid.querySelector<HTMLButtonElement>(
				`[data-index="${target}"] .open-screenshot`,
			)?.focus();
	}

	async function handleDelete(path: string) {
		if (deleting) return;
		const id = instance.uuid;
		const token = request;
		const filename = path.split(/[\\/]/).pop();
		if (
			filename &&
			confirm(`¿Estás seguro de que deseas eliminar esta captura?`)
		) {
			deleting = path;
			try {
				await deleteInstanceFile(id, "screenshots", filename, true);
				if (!alive || token !== request) return;
				if (selectedImage === path) selectedImage = null;
				deleting = null;
				await loadScreenshots();
			} catch (err) {
				if (alive && token === request) {
					error = String(err);
					deleting = null;
				}
			}
		}
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key === "Escape") selectedImage = null;
	}}
/>

<div class="screenshots-section">
	<div class="section-header">
		<span class="section-title"
			>{t("instanceView.screenshots.title")} ({screenshots.length})</span
		>
	</div>

	{#if error}<p role="alert">
			{error}
			<button type="button" onclick={() => loadScreenshots()}
				>{t("common.retry")}</button
			>
		</p>{/if}
	<div
		class="screenshots-grid"
		bind:this={grid}
		onscroll={handleScroll}
		style:--screenshot-columns={columns}
	>
		{#if loading}<p role="status">{t("common.loading")}</p>{/if}
		<div
			class="screenshot-space"
			style:height={`${Math.max(0, rowCount * rowHeight - metrics.gap)}px`}
		>
			{#each rows as row (row)}
				<div
					class="screenshot-row"
					style:transform={`translateY(${row * rowHeight}px)`}
					style:height={`${rowHeight - metrics.gap}px`}
				>
					{#each screenshots.slice(row * columns, (row + 1) * columns) as path, offset (path)}
						{@const index = row * columns + offset}
						{@const preview = thumbnails.get(path)}
						<div class="screenshot-card" data-index={index}>
							<button
								type="button"
								class="open-screenshot"
								onclick={() => (selectedImage = path)}
								onkeydown={(event) => navigate(event, index)}
								aria-label={path.split(/[\\/]/).pop()}
							>
								{#if preview}<img
										src={preview}
										alt=""
										decoding="async"
									/>{:else}<Icon
										name="instance:image"
										size={24}
									/>{/if}
							</button>
							<div class="overlay">
								<button
									type="button"
									class="delete-btn"
									disabled={deleting !== null || loading}
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
				</div>
			{/each}
		</div>

		{#if !loading && !error && screenshots.length === 0}
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
			onkeydown={(e) => {
				if (e.key === "Escape") selectedImage = null;
				e.stopPropagation();
			}}
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
		flex: 1;
		min-height: 0;
		min-width: 0;
	}

	.section-title {
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--text-primary);
	}

	.screenshots-grid {
		--screenshot-min-width: 200px;
		--screenshot-gap: 1rem;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overflow-anchor: none;
		scrollbar-gutter: stable;
		position: relative;
	}
	.screenshot-space {
		position: relative;
		width: 100%;
	}
	.screenshot-row {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		display: grid;
		grid-template-columns: repeat(
			var(--screenshot-columns),
			minmax(0, 1fr)
		);
		gap: var(--screenshot-gap);
	}
	.open-screenshot {
		width: 100%;
		height: 100%;
		border: 0;
		padding: 0;
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.screenshot-card {
		position: relative;
		min-width: 0;
		height: 100%;
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

	.screenshot-card:hover .overlay,
	.screenshot-card:focus-within .overlay {
		opacity: 1;
	}

	.delete-btn {
		background: var(--media-overlay);
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
		color: var(--media-overlay-text);
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
		background: var(--viewer-overlay);
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
		box-shadow: var(--shadow-floating);
	}

	.close-btn {
		position: absolute;
		top: -40px;
		right: -40px;
		background: transparent;
		border: none;
		color: var(--media-overlay-text);
		font-size: 2rem;
		cursor: pointer;
	}

	@media (max-width: 700px) {
		.screenshots-grid {
			--screenshot-min-width: 160px;
		}
	}

	@media (max-width: 550px) {
		.screenshots-grid {
			--screenshot-min-width: 130px;
			--screenshot-gap: 8px;
		}
	}

	@media (max-width: 400px) {
		.screenshots-grid {
			--screenshot-min-width: 100px;
			--screenshot-gap: 6px;
		}
	}
</style>
