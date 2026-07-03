<!--
Este codigo no esta apto para prod !!!!
ATT:SANTIAGOLXX
-->

<script lang="ts">
	import {
		getInstanceResourcePacks,
		getInstanceShaderPacks,
		type ModDownloadInfo,
	} from "$lib/api/cubicApi";
	import { type ModDto } from "$lib/types/types";
	import { SvelteSet } from "svelte/reactivity";
	import Review from "./Review.svelte";
	import Browse from "./Browse.svelte";
	import List from "./List.svelte";

	type ContentType = "resourcepacks" | "shaders";

	let {
		instanceId,
		gameVersion,
		contentType = $bindable("resourcepacks" as ContentType),
		supportsShaders = false,
	} = $props<{
		instanceId: string;
		gameVersion?: string;
		loader?: string;
		contentType?: ContentType;
		supportsShaders?: boolean;
	}>();

	let packs = $state<ModDto[]>([]);
	let isLoading = $state(false);
	let prevInstanceId = $state<string>("");

	let mode = $state<"list" | "browse" | "review">("list");

	// IGNORAR

	let downloading = $state(false);
	let downloadQueue = $state<ModDownloadInfo[]>([]);
	let installedPackNames = new SvelteSet<string>();

	const cleanGameVersion = $derived(
		gameVersion ? getGameVersion(gameVersion) : undefined,
	);

	async function loadPacks() {
		if (instanceId) {
			isLoading = true;
			packs =
				contentType === "shaders"
					? await getInstanceShaderPacks(instanceId)
					: await getInstanceResourcePacks(instanceId);
			installedPackNames.clear();
			// installedPackNames = packs.map((p) => p.name.toLowerCase());
			for (const p of packs) {
				installedPackNames.add(p.name.toLocaleLowerCase());
			}
			isLoading = false;
		}
	}

	$effect(() => {
		if (instanceId !== prevInstanceId) {
			prevInstanceId = instanceId;
			loadPacks();
		}
	});

	const i18nPrefix = $derived(
		contentType === "shaders"
			? "instanceView.shaders"
			: "instanceView.resources",
	);

	function getGameVersion(versionStr: string): string {
		const lower = versionStr.toLowerCase();
		if (
			lower.includes("-forge-") ||
			lower.includes("-neoforge-") ||
			lower.includes("-quilt-")
		) {
			for (const sep of ["-forge-", "-neoforge-", "-quilt-"]) {
				const idx = lower.indexOf(sep);
				if (idx !== -1) return versionStr.slice(0, idx);
			}
		}
		if (lower.startsWith("fabric-loader-")) {
			const lastDash = versionStr.lastIndexOf("-");
			if (lastDash !== -1) return versionStr.slice(lastDash + 1);
		}
		return versionStr;
	}
</script>

{#if mode === "review"}
	<Review
		bind:downloadQueue
		{i18nPrefix}
		bind:mode
		{instanceId}
		{contentType}
		bind:downloading
	/>
{:else if mode === "browse"}
	<Browse
		bind:mode
		{i18nPrefix}
		bind:downloadQueue
		{cleanGameVersion}
		{installedPackNames}
		{contentType}
	/>
{:else}
	<List
		{supportsShaders}
		bind:contentType
		{instanceId}
		{isLoading}
		{installedPackNames}
		{i18nPrefix}
		{packs}
		bind:mode
	/>
{/if}
