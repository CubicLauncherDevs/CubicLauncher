<script lang="ts">
	import { INSTANCE_LOGOS, getPreviewIconSrc } from "$lib/icons/logos";
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import { open as openDialog } from "@tauri-apps/plugin-dialog";

	let {
		selectedIcon = $bindable<string | null>(null),
		disabled = false,
		onupload,
		onselect,
	}: {
		selectedIcon: string | null;
		disabled?: boolean;
		onupload?: (path: string) => void;
		onselect?: () => void;
	} = $props();

	const uid = $props.id();
	let open = $state(false);
	let uploading = $state(false);
	let trigger: HTMLButtonElement;
	let panel: HTMLDivElement | undefined;

	function close(restoreFocus = false) {
		open = false;
		if (restoreFocus) trigger?.focus({ preventScroll: true });
	}

	// Render outside the scrolling modal so the chooser is never clipped.
	function portal(node: HTMLDivElement) {
		panel = node;
		document.body.appendChild(node);
		const rect = trigger.getBoundingClientRect();
		const margin = 12;
		const bounds = node.getBoundingClientRect();
		const below = window.innerHeight - rect.bottom - margin;
		const above = rect.top - margin;
		const opensAbove = below < bounds.height + 8 && above > below;
		node.style.left = `${Math.max(margin, Math.min(rect.left, window.innerWidth - bounds.width - margin))}px`;
		node.style.maxHeight = `${Math.max(0, (opensAbove ? above : below) - 8)}px`;
		if (opensAbove)
			node.style.bottom = `${window.innerHeight - rect.top + 8}px`;
		else node.style.top = `${rect.bottom + 8}px`;
		node.querySelector<HTMLButtonElement>(
			"[aria-pressed='true'], .icon-option",
		)?.focus({ preventScroll: true });
		return {
			destroy() {
				panel = undefined;
				node.remove();
			},
		};
	}

	$effect(() => {
		if (disabled) close();
	});
	$effect(() => {
		if (!open) return;
		const outside = (event: Event) => {
			const target = event.target as Node;
			if (!trigger.contains(target) && !panel?.contains(target)) close();
		};
		const escape = (event: KeyboardEvent) => {
			if (event.key !== "Escape") return;
			event.preventDefault();
			event.stopPropagation();
			close(true);
		};
		const scroll = (event: Event) => {
			if (!panel?.contains(event.target as Node)) close();
		};
		const resize = () => close();
		window.addEventListener("pointerdown", outside, true);
		window.addEventListener("focusin", outside);
		window.addEventListener("keydown", escape, true);
		window.addEventListener("scroll", scroll, true);
		window.addEventListener("resize", resize);
		return () => {
			window.removeEventListener("pointerdown", outside, true);
			window.removeEventListener("focusin", outside);
			window.removeEventListener("keydown", escape, true);
			window.removeEventListener("scroll", scroll, true);
			window.removeEventListener("resize", resize);
		};
	});

	function select(path: string | null) {
		selectedIcon = path;
		onselect?.();
		close(true);
	}

	async function handleUpload() {
		if (disabled || uploading) return;
		close(true);
		uploading = true;
		try {
			const selected = await openDialog({
				multiple: false,
				filters: [
					{
						name: t("createInstance.iconFilter"),
						extensions: ["png", "jpg", "jpeg", "webp", "gif"],
					},
				],
			});
			if (selected) onupload?.(selected);
		} catch (e) {
			console.error("Error selecting icon:", e);
		} finally {
			uploading = false;
		}
	}
</script>

<button
	bind:this={trigger}
	type="button"
	class="icon-trigger"
	class:active={open}
	disabled={disabled || uploading}
	aria-label={t("createInstance.iconLabel")}
	aria-haspopup="dialog"
	aria-expanded={open}
	aria-controls={open ? uid : undefined}
	title={t("createInstance.iconLabel")}
	onclick={() => (open = !open)}
>
	<img src={getPreviewIconSrc(selectedIcon)} alt="" />
</button>

{#if open}
	<div
		use:portal
		id={uid}
		class="icon-popover"
		role="dialog"
		aria-label={t("createInstance.iconLabel")}
		tabindex="-1"
	>
		<span class="picker-label">{t("createInstance.iconLabel")}</span>
		<div class="icon-grid">
			{#each INSTANCE_LOGOS as iconName (iconName)}
				{@const iconPath = `/images/instances/${iconName}`}
				<button
					type="button"
					class="icon-option"
					class:selected={selectedIcon === iconPath}
					aria-pressed={selectedIcon === iconPath}
					title={iconName.replace(/\.\w+$/, "")}
					onclick={() => select(iconPath)}
				>
					<img src={iconPath} alt={iconName.replace(/\.\w+$/, "")} />
				</button>
			{/each}
		</div>
		<div class="picker-actions">
			<button type="button" class="upload-btn" onclick={handleUpload}>
				<Icon name="ui:upload" size={14} />
				{t("createInstance.uploadIcon")}
			</button>
			{#if selectedIcon}
				<button
					type="button"
					class="remove-btn"
					onclick={() => select(null)}
					title={t("createInstance.removeIcon")}
					aria-label={t("createInstance.removeIcon")}
				>
					<Icon name="ui:trash" size={14} />
				</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	.icon-trigger {
		width: 88px;
		height: 88px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
		background: var(--bg-input);
		cursor: pointer;
		transition:
			border-color 0.15s,
			background-color 0.15s;
	}
	.icon-trigger img,
	.icon-option img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.icon-trigger:hover:not(:disabled),
	.icon-trigger.active {
		border-color: var(--accent);
		background: var(--bg-item-active);
	}
	.icon-trigger:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.icon-trigger:focus-visible,
	.icon-popover button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.icon-popover {
		position: fixed;
		z-index: 1100;
		box-sizing: border-box;
		width: min(248px, calc(100vw - 24px));
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		overflow-y: auto;
		background: var(--bg-card);
		color: var(--text-primary);
		border: 1px solid var(--border);
		border-radius: var(--border-radius-lg, 8px);
		box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.3));
	}
	.picker-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-secondary);
	}
	.icon-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 6px;
	}
	.icon-option {
		aspect-ratio: 1;
		min-width: 0;
		padding: 8px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-input);
		cursor: pointer;
	}
	.icon-option:hover,
	.icon-option.selected {
		border-color: var(--accent);
		background: rgba(var(--accent-rgb), 0.1);
	}
	.picker-actions {
		display: flex;
		gap: 6px;
		border-top: 1px solid var(--border);
		padding-top: 12px;
	}
	.picker-actions button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		min-height: 32px;
		padding: 6px 10px;
		font: inherit;
		font-size: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-item-active);
		color: var(--text-primary);
		cursor: pointer;
	}
	.upload-btn {
		flex: 1;
	}
	.picker-actions button:hover {
		border-color: var(--accent);
	}
</style>
