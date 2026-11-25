<script lang="ts">
	import { onDestroy, tick, untrack } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { ask } from "@tauri-apps/plugin-dialog";
	import type { InstanceDto } from "#lib/types/types.js";
	import {
		analyzeInstanceCrash,
		diagnoseInstance,
		listInstanceCrashReports,
		type CrashAnalysis,
		type CrashReportEntry,
		type DiagnosticAction,
		type DiagnosticCheck,
		type DiagnosticReport,
	} from "#lib/api/diagnostics.js";
	import {
		hasCrashCheck,
		sortChecks,
		summarize,
	} from "#lib/api/diagnostics.js";
	import { t } from "#lib/i18n/index.js";
	import { showError } from "#lib/state/state.svelte.js";
	import {
		cacheCrashReports,
		cacheDiagnosticReport,
		isDiagnosticReportFresh,
		readInstanceDiagnostics,
	} from "#lib/state/instanceDiagnostics.js";
	import { openInstanceDir } from "#lib/api/cubicApi.js";
	import Icon from "#lib/icons/Icon.svelte";
	import Loading from "#lib/icons/Loading.svelte";
	import CrashAnalysisView from "./CrashAnalysis.svelte";

	const REPORT_MAX_AGE_MS = 5 * 60_000;

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
	let analyzingName = $state<string | null>(null);
	let analysisAttempted = $state(false);
	let analysisEl = $state<HTMLDivElement>();
	let lastStatus = $state<string>("");
	let lastUuid = $state<string>("");
	let alive = true;
	let request = 0;
	let diagnosisRequest = 0;
	let analysisRequest = 0;

	// Cada vez que se abre la sección: datos del caché, crash reports y, si el
	// diagnóstico se guardó antes de la última sesión, se rehace. Un crash solo
	// existe después de jugar, así que su panel debe mirar el estado de ahora.
	$effect(() => {
		if (!active) return;
		const id = instance.uuid;
		untrack(() => {
			const cached = readInstanceDiagnostics(id);
			if (cached) {
				report = cached.report;
				reports = cached.reports;
			}
			void loadReports(id);
			// El caché es de esta sesión de la app y un informe "todo bien" de
			// hace 5 minutos puede ser anterior al crash que acaba de cerrar el
			// juego: en una instancia detenida se rehace siempre.
			const idle =
				instance.status === "off" || instance.status === "error";
			if (
				!cached?.report ||
				idle ||
				!isDiagnosticReportFresh(cached.report, REPORT_MAX_AGE_MS)
			) {
				void runDiagnosis(id);
			}
			void analyzeLatest();
		});
	});

	// Al cambiar la instancia se descarta el análisis de la anterior.
	$effect(() => {
		if (instance.uuid !== lastUuid) {
			lastUuid = instance.uuid;
			if (
				instance.status !== "started" &&
				instance.status !== "starting"
			) {
				lastStatus = instance.status;
			}
			analysis = null;
			analysisName = null;
			analyzingName = null;
			analysisAttempted = false;
			report = null;
		}
	});

	// El juego solo puede crashear mientras corre: cuando el estado deja de ser
	// "started" se refresca el diagnóstico para no mostrar un "todo bien"
	// anterior al crash.
	$effect(() => {
		const wasRunning =
			lastStatus === "started" || lastStatus === "starting";
		const isIdle = instance.status === "off" || instance.status === "error";
		lastStatus = instance.status;
		// Un ciclo connected→off que dura pocos ms es un cambio de estado del
		// arranque, no un crash: sin esperar, cada arranque rehace el diagnóstico.
		if (wasRunning && isIdle && !running) {
			untrack(() => void runDiagnosis(instance.uuid));
		}
	});

	onDestroy(() => {
		alive = false;
		request++;
		diagnosisRequest++;
		analysisRequest++;
	});

	const checks = $derived(report ? sortChecks(report.checks) : []);
	const summary = $derived(summarize(report));
	// El resumen del chequeo no puede decir "todo bien" si el último arranque
	// terminó en crash: el diagnóstico del crash está debajo.
	const crashed = $derived(
		!!report &&
			(hasCrashCheck(checks) ||
				!!analysis?.findings.some(
					(finding) => finding.code !== "unknown",
				)),
	);
	const activeReportName = $derived(
		analysisName ?? analysis?.reportName ?? null,
	);

	async function runDiagnosis(id = instance.uuid) {
		if (running) return;
		const token = ++diagnosisRequest;
		running = true;
		error = "";
		try {
			const result = await diagnoseInstance(id);
			if (alive && token === diagnosisRequest) {
				report = result;
				cacheDiagnosticReport(id, result);
			}
		} catch (err) {
			if (alive && token === diagnosisRequest) error = String(err);
		} finally {
			if (alive && token === diagnosisRequest) running = false;
		}
	}

	async function loadReports(id = instance.uuid) {
		const token = ++request;
		try {
			const result = await listInstanceCrashReports(id);
			if (alive && token === request) {
				reports = result;
				cacheCrashReports(id, result);
			}
		} catch {
			if (alive && token === request) reports = [];
		}
	}

	async function analyzeLatest() {
		if (analyzing || analysisAttempted) return;
		analysisAttempted = true;
		await analyze();
	}

	async function analyze(name?: string) {
		if (analyzing) return;
		const id = instance.uuid;
		const token = ++analysisRequest;
		analyzing = true;
		analyzingName = name ?? null;
		error = "";
		try {
			const result = await analyzeInstanceCrash(id, name);
			if (alive && token === analysisRequest) {
				analysis = result;
				analysisName = name ?? null;
				if (name) {
					await tick();
					analysisEl?.scrollIntoView({ block: "nearest" });
				}
			}
		} catch (err) {
			if (alive && token === analysisRequest) error = String(err);
		} finally {
			if (alive && token === analysisRequest) {
				analyzing = false;
				analyzingName = null;
			}
		}
	}

	function checkParams(check: DiagnosticCheck) {
		// Un crash sin causa detectada no se titula "Crash no reconocido": el
		// título ya dice que el juego se cerró.
		if (check.params.cause === "unknown") return {};
		return {
			...check.params,
			...(check.params.cause
				? {
						cause: t(
							`diagnostics.crash.findings.${check.params.cause}`,
						),
					}
				: {}),
		};
	}

	function checkTitle(check: DiagnosticCheck): string {
		const key = `diagnostics.checks.${check.id}`;
		const label = t(key, checkParams(check));
		return label === key
			? t("diagnostics.crash.unknownCode", { code: check.id })
			: label;
	}

	function actionLabel(action: DiagnosticAction): string {
		const key = `diagnostics.actions.${action.kind}`;
		const params =
			action.kind === "installJava"
				? { version: action.version }
				: undefined;
		const label = t(key, params);
		return label === key ? t("diagnostics.actions.generic") : label;
	}

	async function runAction(action: DiagnosticAction) {
		switch (action.kind) {
			case "reinstallVersion": {
				const confirmed = await ask(
					t("diagnostics.actions.reinstallConfirm", {
						version: action.version,
					}),
					{
						title: t("diagnostics.title"),
						kind: "warning",
						okLabel: t("diagnostics.actions.reinstallVersion"),
						cancelLabel: t("common.cancel"),
					},
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
					await runDiagnosis();
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
			onclick={() => runDiagnosis()}
		>
			{running ? t("diagnostics.running") : t("diagnostics.run")}
		</button>
	</div>

	{#if error}<p class="error" role="alert">{error}</p>{/if}

	<div class="body">
		<div class="main">
			{#if report}
				<p
					class="summary"
					class:ok={summary.label === "allGood" && !crashed}
					class:bad={summary.label === "issues" || crashed}
					role="status"
				>
					{#if crashed && summary.label === "allGood"}
						{t("diagnostics.issues", {
							errors: summary.errors,
							warnings: Math.max(summary.warnings, 1),
						})}
					{:else if summary.label === "allGood"}
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
							<li
								class="check"
								class:error={check.severity === "error"}
							>
								<div class="check-head">
									<span
										class="severity-dot"
										aria-hidden="true"
									></span>
									<span class="check-title"
										>{checkTitle(check)}</span
									>
									{#if check.action}
										<button
											type="button"
											class="fix"
											onclick={() =>
												check.action &&
												runAction(check.action)}
											>{actionLabel(check.action)}</button
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

			{#if analysis && (analysis.reportName || analysis.findings.length > 0)}
				<div class="analysis" bind:this={analysisEl}>
					<CrashAnalysisView
						{analysis}
						loading={analyzing}
						onanalyze={() => analyze(analysisName ?? undefined)}
						onaction={runAction}
					/>
				</div>
			{:else if analysisAttempted && !analyzing}
				<p class="hint">{t("diagnostics.crash.noKnownCause")}</p>
			{/if}
		</div>

		<aside class="reports" aria-label={t("diagnostics.reports.title")}>
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
						<li
							class="report-row"
							class:selected={entry.name === activeReportName}
						>
							<div class="report-info">
								<span class="report-name" title={entry.name}
									>{entry.name}</span
								>
								<span class="report-meta"
									>{formatDate(entry.modified)} · {formatBytes(
										entry.size,
									)}</span
								>
							</div>
							<button
								type="button"
								disabled={analyzing}
								aria-busy={analyzing &&
									analyzingName === entry.name}
								onclick={() => analyze(entry.name)}
							>
								{#if analyzing && analyzingName === entry.name}
									<Loading size={12} />
								{:else}
									{t("diagnostics.reports.analyze")}
								{/if}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</aside>
	</div>
</section>

<style>
	.diagnostics {
		display: flex;
		flex-direction: column;
		gap: 16px;
		color: var(--text-primary);
	}
	.body {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(260px, 320px);
		gap: 16px;
		align-items: start;
	}
	.main {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-width: 0;
	}
	.analysis {
		min-width: 0;
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
		margin: 0;
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
		background: var(--accent);
		border-color: var(--accent);
		color: var(--accent-text);
		font-weight: 600;
	}
	button.primary:hover:not(:disabled) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
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
	.check {
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
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--border-radius);
		background: var(--bg-card);
	}
	.report-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		max-height: 50vh;
		overflow-y: auto;
	}
	.report-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border: 1px solid transparent;
		border-radius: var(--border-radius-sm);
		background: var(--surface-input);
	}
	.report-row.selected {
		border-color: color-mix(
			in srgb,
			var(--accent-primary) 45%,
			var(--border)
		);
		background: var(--surface-active);
	}
	.report-info {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.report-name {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-family: var(--font-family-mono);
		font-size: 0.76rem;
	}
	.report-meta {
		color: var(--text-secondary);
		font-size: 0.7rem;
		white-space: nowrap;
	}
	.report-row button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		min-width: 72px;
		padding: 5px 10px;
		font-size: 0.74rem;
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
	@media (max-width: 860px) {
		.body {
			grid-template-columns: 1fr;
		}
	}
</style>
