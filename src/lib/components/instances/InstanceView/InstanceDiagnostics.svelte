<script lang="ts">
	import { onDestroy } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import type { InstanceDto } from "$lib/types/types";
	import {
		analyzeInstanceCrash,
		diagnoseInstance,
		listInstanceCrashReports,
		type CrashAnalysis,
		type CrashReportEntry,
		type DiagnosticAction,
		type DiagnosticCheck,
		type DiagnosticReport,
	} from "$lib/api/diagnostics";
	import { summarize, sortChecks } from "$lib/api/diagnostics";
	import { t } from "$lib/i18n";
	import { showError } from "$lib/state/state.svelte";
	import { openInstanceDir } from "$lib/api/cubicApi";
	import Icon from "$lib/icons/Icon.svelte";
	import CrashAnalysisView from "./CrashAnalysis.svelte";

	let {
		instance,
		active = true,
		onopenmods,
		onopenworlds,
	}: {
		instance: InstanceDto;
		active?: boolean;
		onopenmods: () => void;
		onopenworlds: () => void;
	} = $props();

	let report = $state<DiagnosticReport | null>(null);
	let running = $state(false);
	let error = $state("");
	let reports = $state<CrashReportEntry[]>([]);
	let analysis = $state<CrashAnalysis | null>(null);
	let analysisName = $state<string | null>(null);
	let analyzing = $state(false);
	let alive = true;
	let request = 0;

	$effect(() => {
		if (!active) return;
		void loadReports();
	});

	onDestroy(() => {
		alive = false;
		request++;
	});

	const checks = $derived(report ? sortChecks(report.checks) : []);
	const summary = $derived(summarize(report));

	async function runDiagnosis() {
		if (running) return;
		running = true;
		error = "";
		try {
			const result = await diagnoseInstance(instance.uuid);
			if (alive) report = result;
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) running = false;
		}
	}

	async function loadReports() {
		const token = ++request;
		try {
			const result = await listInstanceCrashReports(instance.uuid);
			if (alive && token === request) reports = result;
		} catch {
			if (alive && token === request) reports = [];
		}
	}

	async function analyze(name?: string) {
		if (analyzing) return;
		analyzing = true;
		error = "";
		try {
			const result = await analyzeInstanceCrash(instance.uuid, name);
			if (alive) {
				analysis = result;
				analysisName = name ?? null;
			}
		} catch (err) {
			if (alive) error = String(err);
		} finally {
			if (alive) analyzing = false;
		}
	}

	function checkTitle(check: DiagnosticCheck): string {
		const key = `diagnostics.checks.${check.id}`;
		const label = t(key, check.params);
		return label === key
			? t("diagnostics.crash.unknownCode", { code: check.id })
			: label;
	}

	async function runAction(action: DiagnosticAction) {
		switch (action.kind) {
			case "reinstallVersion": {
				const confirmed = window.confirm(
					t("diagnostics.actions.reinstallConfirm", {
						version: action.version,
					}),
				);
				if (!confirmed) return;
				try {
					await invoke("reinstall_version", {
						version: action.version,
					});
					await runDiagnosis();
				} catch (err) {
					showError(t("errors.title"), String(err));
				}
				return;
			}
			case "installJava":
				try {
					await invoke("install_jre", {
						version: action.version,
						vendor: null,
					});
				} catch (err) {
					showError(t("errors.title"), String(err));
				}
				return;
			case "openMods":
				onopenmods();
				return;
			case "openWorlds":
				onopenworlds();
				return;
		}
	}

	async function openReports() {
		try {
			await openInstanceDir(instance.uuid, "crash-reports");
		} catch (err) {
			showError(t("errors.title"), String(err));
		}
	}

	function formatDate(ms: number): string {
		if (!ms) return t("diagnostics.reports.unknownDate");
		const date = new Date(ms);
		return Number.isNaN(date.getTime())
			? t("diagnostics.reports.unknownDate")
			: date.toLocaleString();
	}

	function formatBytes(bytes: number): string {
		const unit = Math.min(
			3,
			Math.floor(Math.log2(Math.max(1, bytes)) / 10),
		);
		return `${(bytes / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KiB", "MiB", "GiB"][unit]}`;
	}
</script>

<section class="diagnostics" aria-label={t("diagnostics.title")}>
	<div class="toolbar">
		<h2>
			<Icon name="instance:check-square" size={20} />
			{t("diagnostics.title")}
		</h2>
		<button
			type="button"
			class="primary"
			disabled={running}
			onclick={runDiagnosis}
		>
			{running ? t("diagnostics.running") : t("diagnostics.run")}
		</button>
	</div>

	{#if error}<p class="error" role="alert">{error}</p>{/if}

	{#if report}
		<p
			class="summary"
			class:ok={summary.label === "allGood"}
			class:bad={summary.label === "issues"}
			role="status"
		>
			{#if summary.label === "allGood"}
				{t("diagnostics.allGood")}
			{:else}
				{t("diagnostics.issues", {
					errors: summary.errors,
					warnings: summary.warnings,
				})}
			{/if}
		</p>

		{#if checks.length > 0}
			<ul class="checks">
				{#each checks as check (check.id)}
					<li class="check" class:error={check.severity === "error"}>
						<div class="check-head">
							<span class="severity-dot" aria-hidden="true"
							></span>
							<span class="check-title">{checkTitle(check)}</span>
							{#if check.action}
								<button
									type="button"
									class="fix"
									onclick={() =>
										check.action && runAction(check.action)}
									>{t(
										`diagnostics.actions.${check.action.kind}`,
									)}</button
								>
							{/if}
						</div>
						{#if check.details.length > 0}
							<ul class="details">
								{#each check.details as detail (detail)}
									<li><code>{detail}</code></li>
								{/each}
							</ul>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	{/if}

	<div class="reports">
		<div class="reports-head">
			<h3>{t("diagnostics.reports.title")}</h3>
			<button type="button" class="ghost" onclick={openReports}
				>{t("diagnostics.reports.openFolder")}</button
			>
		</div>
		{#if reports.length === 0}
			<p class="hint">{t("diagnostics.reports.empty")}</p>
		{:else}
			<ul class="report-list">
				{#each reports as entry (entry.name)}
					<li class="report-row">
						<span class="report-name">{entry.name}</span>
						<span class="report-meta"
							>{formatDate(entry.modified)} · {formatBytes(
								entry.size,
							)}</span
						>
						<button
							type="button"
							disabled={analyzing}
							onclick={() => analyze(entry.name)}
							>{t("diagnostics.reports.analyze")}</button
						>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	{#if analysis}
		<CrashAnalysisView
			{analysis}
			loading={analyzing}
			onanalyze={() => analyze(analysisName ?? undefined)}
			onaction={runAction}
		/>
	{/if}
</section>

<style>
	.diagnostics {
		display: flex;
		flex-direction: column;
		gap: 16px;
		color: var(--text-primary);
	}
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}
	h2 {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0;
		font-size: 1.15rem;
	}
	h3 {
		margin: 0 0 8px;
		font-size: 0.95rem;
	}
	.reports-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}
	.ghost {
		font-size: 0.75rem;
		padding: 4px 9px;
		color: var(--text-secondary);
	}
	button {
		font: inherit;
		font-size: 0.82rem;
		border: 1px solid var(--border);
		border-radius: var(--border-radius-sm);
		background: var(--bg-card);
		color: var(--text-primary);
		padding: 8px 12px;
		cursor: pointer;
	}
	button:hover:not(:disabled) {
		background: var(--bg-item-active);
		border-color: var(--accent-primary);
	}
	button:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	button.primary {
		background: var(--accent-primary);
		border-color: var(--accent-primary);
		color: var(--media-overlay-text, #fff);
		font-weight: 600;
	}
	.summary {
		margin: 0;
		padding: 10px 14px;
		border-radius: var(--border-radius);
		border: 1px solid var(--border);
		background: var(--bg-card);
	}
	.summary.ok {
		border-color: color-mix(
			in srgb,
			var(--accent-primary) 50%,
			var(--border)
		);
	}
	.summary.bad {
		border-color: color-mix(in srgb, var(--log-error) 50%, var(--border));
	}
	.checks,
	.report-list,
	.details {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.checks {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.check,
	.report-row {
		padding: 10px 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}
	.check-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.severity-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--accent-primary);
		flex-shrink: 0;
	}
	.check.error .severity-dot {
		background: var(--log-error);
	}
	.check-title {
		flex: 1;
		font-weight: 600;
	}
	.fix {
		font-size: 0.74rem;
		padding: 5px 10px;
	}
	.details {
		margin-top: 6px;
		padding-left: 16px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		color: var(--text-secondary);
		font-size: 0.78rem;
	}
	.details code {
		font-family: var(--font-family-mono);
		overflow-wrap: anywhere;
	}
	.reports {
		border-top: 1px solid var(--border);
		padding-top: 14px;
	}
	.report-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 6px;
	}
	.report-name {
		flex: 1;
		overflow-wrap: anywhere;
		font-family: var(--font-family-mono);
		font-size: 0.78rem;
	}
	.report-meta {
		color: var(--text-secondary);
		font-size: 0.76rem;
		white-space: nowrap;
	}
	.hint {
		margin: 0;
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.error {
		margin: 0;
		color: var(--log-error);
		overflow-wrap: anywhere;
	}
	@media (max-width: 600px) {
		.report-row {
			flex-wrap: wrap;
		}
	}
</style>
