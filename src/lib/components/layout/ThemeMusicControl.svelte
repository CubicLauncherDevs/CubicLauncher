<script lang="ts">
	import Icon from "$lib/icons/Icon.svelte";
	import { t } from "$lib/i18n";
	import { themeMusic } from "$lib/api/themeManager";

	let audio = $state<HTMLAudioElement | null>(null);
	let paused = $state(true);
	let loadedSource = $state<string | null>(null);
	const source = $derived(themeMusic.get("source") ?? null);

	$effect(() => {
		const element = audio;
		const nextSource = source;
		if (!element || !nextSource || loadedSource === nextSource) return;

		loadedSource = nextSource;
		paused = true;
		element.volume = 0.35;
		element.load();
		void element.play().catch(() => {
			if (audio === element) paused = true;
		});
	});

	function togglePlayback() {
		if (!audio) return;
		if (audio.paused) {
			void audio.play().catch(() => {
				paused = true;
			});
		} else {
			audio.pause();
		}
	}
</script>

{#if source}
	<audio
		bind:this={audio}
		src={source}
		autoplay
		loop
		preload="auto"
		onplay={() => (paused = false)}
		onpause={() => (paused = true)}
		onerror={() => (paused = true)}
	></audio>
	<button
		type="button"
		class="theme-music-control"
		onclick={togglePlayback}
		aria-label={t(paused ? "themes.music.resume" : "themes.music.pause")}
		aria-pressed={!paused}
		title={t(paused ? "themes.music.resume" : "themes.music.pause")}
	>
		<Icon name={paused ? "ui:play" : "ui:pause"} size={17} />
	</button>
{/if}

<style>
	audio {
		display: none;
	}

	.theme-music-control {
		position: fixed;
		right: var(--space-md, 16px);
		bottom: var(--space-md, 16px);
		z-index: 20;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 38px;
		height: 38px;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: 50%;
		background: var(--bg-card-gradient, var(--bg-card));
		color: var(--text-primary);
		box-shadow: var(--shadow-md, var(--shadow-sm));
		cursor: pointer;
		transition:
			background 0.15s ease,
			border-color 0.15s ease,
			transform 0.15s ease;
	}

	.theme-music-control:hover {
		background: var(--bg-item-active);
		border-color: var(--accent);
		transform: scale(1.04);
	}

	.theme-music-control:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
</style>
