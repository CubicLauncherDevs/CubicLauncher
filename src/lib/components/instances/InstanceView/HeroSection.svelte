<script lang="ts">
	import { t } from "$lib/i18n";

	let {
		instanceName,
		instanceIcon,
		lastPlayedLabel,
		screenshotUrl = null as string | null,
		bannerState = "Idle",
		onPlay,
		onPickBanner,
	}: {
		instanceName: string;
		instanceIcon: string;
		lastPlayedLabel: string;
		screenshotUrl: string | null;
		bannerState: string;
		onPlay: () => void;
		onPickBanner: () => void;
	} = $props();
</script>

<section
	class="hero-section"
	style={screenshotUrl
		? `background-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.2), rgba(0, 0, 0, 0.8)), url(${screenshotUrl})`
		: "background: linear-gradient(180deg, rgba(255, 255, 255, 0.02) 0%, rgba(0, 0, 0, 0) 100%);"}
>
	<img
		class="instance-big-icon"
		src={instanceIcon || "/images/cubic.svg"}
		alt="Icon"
	/>
	<div class="instance-title-area">
		<h2>{instanceName}</h2>
		<div class="last-played">{lastPlayedLabel}</div>
		{#if bannerState == "Started"}
			<button type="button" class="play-btn" onclick={onPlay}>
				{t("instanceView.close")}
			</button>
		{:else if bannerState == "Starting"}
			<button type="button" class="play-btn" disabled>
				{t("instanceView.playBtn")}
			</button>
		{:else}
			<button type="button" class="play-btn" onclick={onPlay}>
				{t("instanceView.playBtn")}
			</button>
		{/if}
	</div>

	<div class="banner-controls">
		<button
			type="button"
			class="banner-btn"
			onclick={onPickBanner}
			title={t("instanceView.changeBannerTitle")}
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
				><path
					d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"
				/><circle cx="12" cy="13" r="4" /></svg
			>
			<span>{t("instanceView.changeBanner")}</span>
		</button>
	</div>
</section>

<style>
	.hero-section {
		padding: 50px 40px;
		display: flex;
		align-items: center;
		gap: 28px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		position: relative;
		background-size: cover;
		background-position: center;
		transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.banner-controls {
		position: absolute;
		top: 20px;
		right: 20px;
		display: flex;
		gap: 8px;
		opacity: 0;
		transition: opacity 0.3s ease;
	}

	.hero-section:hover .banner-controls {
		opacity: 1;
	}

	.banner-btn {
		background: rgba(0, 0, 0, 0.5);
		backdrop-filter: blur(var(--backdrop-blur-button, 4px));
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: white;
		padding: 6px 12px;
		border-radius: var(--border-radius-sm);
		font-size: 0.7rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 6px;
		transition: all 0.2s;
	}

	.banner-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.2);
	}

	.instance-big-icon {
		width: 64px;
		height: 64px;
		border-radius: var(--border-radius-sm);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		filter: drop-shadow(0 0 10px rgba(255, 255, 255, 0.4));
	}

	.instance-title-area {
		display: flex;
		flex-direction: column;
		gap: 8px;
		text-shadow: 0 2px 10px rgba(0, 0, 0, 0.5);
	}

	.instance-title-area h2 {
		font-size: 1.5rem;
		font-weight: 800;
		letter-spacing: -0.5px;
		color: white;
	}

	.last-played {
		color: rgba(255, 255, 255, 0.8);
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 1.5px;
		font-weight: 700;
	}

	.play-btn {
		background: white;
		color: black;
		border: none;
		padding: 10px 28px;
		border-radius: var(--border-radius-sm);
		font-size: 0.8rem;
		font-weight: 800;
		cursor: pointer;
		width: fit-content;
		transition:
			background 0.2s ease,
			box-shadow 0.2s ease;
		letter-spacing: 0.5px;
		text-transform: uppercase;
		box-shadow: 0 4px 15px rgba(0, 0, 0, 0.3);
	}

	.play-btn:hover:not(:disabled) {
		background: #f0f0f0;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
	}

	.play-btn:active:not(:disabled) {
		transform: scale(0.98);
	}

	.play-btn:disabled {
		background: rgba(255, 255, 255, 0.15);
		color: rgba(255, 255, 255, 0.35);
		cursor: not-allowed;
		box-shadow: none;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	@media (max-width: 1024px) {
		.hero-section { padding: 40px; }
	}

	@media (max-width: 950px) {
		.hero-section { padding: 30px 24px; gap: 20px; }
	}

	@media (max-width: 850px) {
		.hero-section { padding: 24px 20px; gap: 16px; }
	}

	@media (max-width: 700px) {
		.hero-section { padding: 20px 16px; gap: 14px; }
		.instance-big-icon { width: 48px; height: 48px; }
		.instance-title-area h2 { font-size: 1.3rem; }
		.banner-controls { opacity: 1; }
	}

	@media (max-width: 650px) {
		.hero-section { flex-direction: column; align-items: center; text-align: center; padding: 24px 16px; }
		.play-btn { margin: 0 auto; }
	}

	@media (max-width: 550px) {
		.hero-section { padding: 16px 12px; gap: 12px; }
		.instance-big-icon { width: 40px; height: 40px; }
		.instance-title-area h2 { font-size: 1.1rem; }
		.play-btn { padding: 8px 20px; font-size: 0.7rem; }
		.last-played { font-size: 0.58rem; }
	}

	@media (max-width: 400px) {
		.hero-section { padding: 12px 8px; gap: 8px; }
		.instance-big-icon { width: 32px; height: 32px; }
		.instance-title-area h2 { font-size: 1rem; }
	}
</style>
