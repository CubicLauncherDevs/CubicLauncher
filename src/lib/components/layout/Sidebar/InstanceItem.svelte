<script lang="ts">
	import type { InstanceDto } from "#lib/types/types.js";
	import { t } from "#lib/i18n/index.js";
	import Icon from "#lib/icons/Icon.svelte";
	import Trash from "#lib/icons/Trash.svelte";
	import { getDisplayIconSrc } from "#lib/icons/logos.js";

	let {
		instance,
		selected,
		onselect,
		onedit,
		ondelete,
	}: {
		instance: InstanceDto;
		selected: boolean;
		onselect: () => void;
		onedit: () => void;
		ondelete: () => void;
	} = $props();

	let isRunning = $derived(instance.status === "started");
	let isPinned = $derived(instance.pinned);
	let iconUrl = $derived(getDisplayIconSrc(instance.icon));
	let formattedLoader = $derived(
		instance.loader.charAt(0).toUpperCase() + instance.loader.slice(1),
	);
	let minecraftVersion = $derived(getMinecraftVersion(instance.version));
	let subtitle = $derived(
		[minecraftVersion, formattedLoader].filter(Boolean).join(" • "),
	);
	let nameViewport: HTMLSpanElement | undefined = $state();
	let nameContent: HTMLSpanElement | undefined = $state();
	let nameScrollDistance = $state(0);
	const nameScrollDuration = $derived(
		Math.min(7000, Math.max(3000, 1500 + nameScrollDistance * 32)),
	);
	const canScrollName = $derived(instance.name.length > 12);

	$effect(() => {
		const viewport = nameViewport;
		const content = nameContent;
		if (!viewport || !content) return;

		const update = () => {
			nameScrollDistance = canScrollName
				? Math.max(28, content.scrollWidth - viewport.clientWidth)
				: 0;
		};
		const observer = new ResizeObserver(update);
		observer.observe(viewport);
		observer.observe(content);
		update();
		return () => observer.disconnect();
	});

	function getMinecraftVersion(version: string): string {
		const fabricOrQuilt = version.match(
			/^(?:fabric|quilt)-loader-[\d.]+-(.+?)(?:-(?:fabric|quilt))?$/,
		);
		if (fabricOrQuilt) return fabricOrQuilt[1];

		for (const separator of ["-neoforge-", "-forge-", "-OptiFine_"]) {
			const index = version.indexOf(separator);
			if (index >= 0) return version.slice(0, index);
		}

		return version;
	}
</script>

<div
	class="instance-item"
	class:active={selected}
	data-instance-uuid={instance.uuid}
	onclick={onselect}
	onkeydown={(e) => {
		if (e.key === "Enter" || e.key === " ") onselect();
	}}
	role="button"
	tabindex="0"
	title={instance.name}
>
	<div class="instance-icon">
		{#if instance.icon}
			<img
				src={iconUrl}
				alt={instance.name}
				loading="lazy"
				decoding="async"
				width="24"
				height="24"
			/>
		{:else}
			<Icon name="brand:cubic" size={24} />
		{/if}
		{#if isRunning}
			<span
				class="status-dot"
				aria-label={t("instanceView.status.started")}
			></span>
		{/if}
	</div>

	<div class="instance-text">
		<span
			class="instance-name"
			class:overflowing={canScrollName}
			bind:this={nameViewport}
		>
			<span
				class="instance-name-content"
				bind:this={nameContent}
				style:--name-scroll-distance={`${nameScrollDistance}px`}
				style:--name-scroll-duration={`${nameScrollDuration}ms`}
				>{instance.name}</span
			>
		</span>
		<span class="instance-subtitle">{subtitle}</span>
	</div>

	<div class="instance-actions">
		<button
			type="button"
			class="action-btn"
			onclick={(e) => {
				e.stopPropagation();
				onedit();
			}}
			title={t("sidebar.rename")}
		>
			<Icon name="nav:edit" size={14} />
		</button>
		<button
			type="button"
			class="action-btn delete"
			onclick={(e) => {
				e.stopPropagation();
				ondelete();
			}}
			title={t("sidebar.delete")}
		>
			<Trash width="14" height="14" />
		</button>
	</div>

	{#if isPinned}
		<span class="pin-badge" aria-hidden="true"
			><Icon name="ui:pin" size={11} /></span
		>
	{/if}
</div>

<style>
	.instance-item {
		/* Color opaco por estado; alimenta el fondo y el degradado de las
		   acciones sin provocar reflujo al hacer hover. */
		--item-bg: var(--bg-sidebar);

		position: relative;
		display: flex;
		align-items: center;
		border-radius: var(--border-radius-lg);
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			box-shadow 0.15s ease;
		border: 1px solid transparent;
		background: transparent;
		color: var(--text-primary);
		width: 100%;
		text-align: left;
		outline: none;
		height: calc(
			var(--virtual-row-height, var(--sidebar-row-height, 52px)) -
				var(--sidebar-row-gap, 8px)
		);
		min-height: 0;
		margin-bottom: 8px;
		box-sizing: border-box;
		overflow: hidden;
		contain: layout paint style;
	}

	.instance-item:hover {
		--item-bg: color-mix(in srgb, var(--surface-rgb) 6%, var(--bg-sidebar));
		background: var(--item-bg);
	}

	.instance-item:focus-visible {
		--item-bg: color-mix(in srgb, var(--surface-rgb) 6%, var(--bg-sidebar));
		background: var(--item-bg);
		border-color: var(--text-secondary);
	}

	.instance-item.active {
		--item-bg: var(--bg-item-active);
		background: var(--item-bg);
		border-color: var(--accent);
		box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.08);
	}

	.instance-icon {
		position: relative;
		flex-shrink: 0;
		width: 24px;
		height: 24px;
		margin-left: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--border-radius-sm);
		color: var(--text-primary);
	}

	.instance-icon img {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: inherit;
		object-fit: contain;
	}

	.pin-badge {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		margin-right: 8px;
		background: var(--bg-sidebar);
		border: 1px solid var(--border);
		border-radius: 50%;
		color: var(--color-warning);
		pointer-events: none;
		box-shadow: var(--shadow-sm);
	}

	.instance-text {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
		padding: 0 10px 0 12px;
	}

	.instance-name {
		position: relative;
		display: block;
		font-weight: 600;
		font-size: var(--sidebar-item-font-size, var(--font-size-control));
		white-space: nowrap;
		overflow: hidden;
	}

	.instance-name-content {
		display: inline-block;
		min-width: max-content;
		box-sizing: border-box;
		padding-inline: 8px;
	}

	.instance-name.overflowing {
		-webkit-mask-image: linear-gradient(
			90deg,
			transparent,
			#000 8px,
			#000 calc(100% - 30px),
			transparent
		);
		mask-image: linear-gradient(
			90deg,
			transparent,
			#000 8px,
			#000 calc(100% - 30px),
			transparent
		);
	}

	.instance-item:hover .instance-name.overflowing .instance-name-content,
	.instance-item.active .instance-name.overflowing .instance-name-content,
	.instance-item:focus-visible
		.instance-name.overflowing
		.instance-name-content {
		animation: instance-name-scroll var(--name-scroll-duration) ease-in-out
			400ms infinite alternate;
	}

	@keyframes instance-name-scroll {
		to {
			transform: translateX(calc(-1 * var(--name-scroll-distance)));
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.instance-item:hover .instance-name.overflowing .instance-name-content,
		.instance-item.active .instance-name.overflowing .instance-name-content,
		.instance-item:focus-visible
			.instance-name.overflowing
			.instance-name-content {
			animation: none;
		}
	}

	.instance-subtitle {
		font-size: 0.7rem;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		transition: color 0.15s ease;
	}

	.instance-item:hover .instance-subtitle,
	.instance-item.active .instance-subtitle {
		color: var(--text-secondary);
	}

	.instance-actions {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.15s ease;
	}

	.instance-item:hover .instance-actions,
	.instance-item.active .instance-actions,
	.instance-item:focus-within .instance-actions {
		opacity: 1;
		pointer-events: auto;
	}

	.action-btn {
		width: 22px;
		height: 22px;
		border-radius: var(--border-radius-sm);
		border: none;
		background: transparent;
		color: var(--text-tertiary);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			transform 0.12s ease;
	}

	.action-btn:hover {
		background: rgba(var(--surface-rgb), 0.08);
		color: var(--text-primary);
	}

	.action-btn:active {
		transform: scale(0.94);
	}

	.action-btn.delete:hover {
		background: rgba(var(--color-error-rgb), 0.12);
		color: var(--color-error);
	}

	.status-dot {
		position: absolute;
		bottom: -1px;
		right: -1px;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-status-started);
		border: 1.5px solid var(--bg-sidebar);
		box-shadow: 0 0 4px rgba(var(--color-success-rgb), 0.5);
	}

	@media (max-width: 650px) {
		.instance-item {
			justify-content: center;
		}

		.instance-text,
		.instance-actions {
			display: none;
		}
	}
</style>
