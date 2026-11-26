<script lang="ts">
	// 동기화 (CLAUDE.md 2-B-3, ADR-011): 파일로 주고받기. 누를 때만 만들고, 합치기 전에 미리보기 → 자동 사본 → 적용
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Cloud, Download, FolderOpen, Info, LogOut, RefreshCw, RotateCcw } from "@lucide/svelte";
	import type { DriveStatus, SyncBackup, SyncPreview, SyncStatus } from "$ipc/bindings";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import SyncConflicts from "./SyncConflicts.svelte";
	import { deviceLabel, shortId as short } from "./deviceLabel";
	import { Badge, Button, Input, Segmented, Select, Spinner, Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, pickFolder, saveTo, SYNC_FILES } from "$shared/ipc/dialog";
	import { dropPaths, extOf } from "$app/drop.svelte";
	import { layout } from "$app/layout.svelte";
	import { clearNativeDriveToken, hasNativeDrive, NativeDriveError, prepareDrive, runWithDriveAuth, unmetered } from "$app/androidDrive";
	import { syncProgressState } from "$app/syncProgress.svelte";
	import { current, ensureLoaded, save as savePart } from "./pluginSettingForm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "sync" };
	let status = $state<SyncStatus | null>(null);
	let includeFiles = $state(true);
	let busy = $state<"export" | "open" | "apply" | null>(null);
	/** 열어 본 묶음 (적용 전) */
	let pending = $state<{ path: string; preview: SyncPreview } | null>(null);

	// ---- 폴더·드라이브 동기화 (2-C·2-D) ----
	let syncing = $state(false);
	let wiping = $state(false);
	const step = $derived(syncProgressState.step);
	const sync = $derived(current()?.sync ?? null);
	const INTERVALS = $derived([
		{ value: "15", label: $_("settings.syncSec.every15m") },
		{ value: "30", label: $_("settings.syncSec.every30m") },
		{ value: "60", label: $_("settings.syncSec.every1h") },
		{ value: "180", label: $_("settings.syncSec.every3h") },
	]);

	onMount(() => {
		void load();
		if (!showAuto) return;
		ensureLoaded();
	});
	// ---- 되돌리기 (2-E) ----
	let backups = $state<SyncBackup[]>([]);
	let conflictsKey = $state(0);
	async function rollback(b: SyncBackup) {
		const ok = await confirmAction({
			title: $_("settings.syncSec.rollbackTitle"),
			message: $_("settings.syncSec.rollbackMsg", { values: { date: absolute(b.createdAt) } }),
			confirmLabel: $_("settings.syncSec.rollbackConfirm"),
			danger: true,
		});
		if (!ok) return;
		await call(commands.requestSyncRollback(b.name), src);
		await call(commands.restartApp(), src);
	}
	async function cancelRollback() {
		await call(commands.cancelSyncRollback(), src);
		toasts.success($_("settings.syncSec.rollbackCancelled"));
		await load();
	}

	async function setExcluded(deviceId: string, excluded: boolean) {
		await call(commands.setSyncPeerExcluded(deviceId, excluded), src);
		await load();
	}
	async function excludePeer(deviceId: string) {
		const ok = await confirmAction({
			title: $_("settings.syncSec.excludeTitle"),
			message: $_("settings.syncSec.excludeMsg", { values: { device: label(deviceId) } }),
			confirmLabel: $_("settings.syncSec.excludeBtn"),
			danger: true,
		});
		if (ok) await setExcluded(deviceId, true);
	}
	async function load() {
		status = await call(commands.getSyncStatus(), src).catch(() => null);
		backups = await call(commands.listSyncBackups(), { ...src, silent: true }).catch(() => []);
		if (showAuto) {
			drive = await call(commands.driveStatus(), { ...src, silent: true }).catch(() => null);
		}
	}
	// ---- 구글 드라이브 (2-D) ----
	// Android는 폰의 Play 서비스가 권한을 준다 (폴더 동기화는 데스크톱만)
	const phone = hasNativeDrive();
	const showAuto = layout.desktop || phone;
	let drive = $state<DriveStatus | null>(null);
	let loggingIn = $state(false);
	const targetKind = $derived((phone || sync?.target === "drive" ? "drive" : "folder") as "folder" | "drive");
	const ready = $derived(targetKind === "drive" ? !!drive?.connected : !!sync?.folder);
	async function loginDrive() {
		loggingIn = true;
		try {
			drive = phone ? await prepareDrive(true, true) : await call(commands.driveLogin(), src);
			toasts.success(drive.email ? $_("settings.syncSec.driveConnectedEmail", { values: { email: drive.email } }) : $_("settings.syncSec.driveConnected"));
		} catch (e) {
			// 백엔드 오류는 call이 이미 토스트로, 폰(Play 서비스)의 오류는 여기서
			if (e instanceof NativeDriveError) toasts.error($_("settings.syncSec.driveConnectError"), e.message);
		} finally {
			loggingIn = false;
		}
	}
	async function logoutDrive() {
		const keep = await confirmAction({
			title: $_("settings.syncSec.driveLogoutTitle"),
			message: $_("settings.syncSec.driveLogoutMsg"),
			confirmLabel: $_("settings.syncSec.driveLogoutBtn"),
		});
		if (!keep) return;
		if (phone) await prepareDrive(false, false).catch(() => null); // 권한을 거두려면 지금 토큰이 필요
		await call(commands.driveLogout(false), src);
		if (phone) await clearNativeDriveToken();
		drive = await call(commands.driveStatus(), { ...src, silent: true }).catch(() => null);
		toasts.success($_("settings.syncSec.driveLoggedOut"));
	}
	async function wipeDrive() {
		const ok = await confirmAction({
			title: $_("settings.syncSec.driveWipeTitle"),
			message: $_("settings.syncSec.driveWipeMsg"),
			confirmLabel: $_("settings.syncSec.driveWipeBtn"),
			danger: true,
		});
		if (!ok) return;
		wiping = true;
		try {
			const count = await runWithDriveAuth(
				true,
				(silent) => call(commands.driveLogout(true), { ...src, silent }),
				{ onReconnected: (s) => (drive = s) },
			);
			if (phone) await clearNativeDriveToken();
			drive = await call(commands.driveStatus(), { ...src, silent: true }).catch(() => null);
			toasts.success($_("settings.syncSec.driveWiped", { values: { count } }));
		} catch (e) {
			if (e instanceof NativeDriveError) {
				toasts.error($_("settings.syncSec.driveWipeFailed"), e.message);
			}
		} finally {
			wiping = false;
			await load();
		}
	}

	async function chooseFolder() {
		const f = await pickFolder();
		if (f) savePart("sync", { folder: f });
	}
	async function syncNow() {
		if (phone && sync?.wifiOnly && !unmetered()) {
			const ok = await confirmAction({ title: $_("settings.syncSec.cellularTitle"), message: $_("settings.syncSec.cellularMsg"), confirmLabel: $_("settings.syncSec.title") });
			if (!ok) return;
		}
		syncing = true;
		try {
			// 하단 고정 팝업(jobs.run)을 띄우지 않고 토스트·백그라운드 알림으로 진행률을 표시한다 (1-J-2).
			// 드라이브 권한이 취소·만료된 경우에는 에러 반환 대신 다시 권한을 받아 이어서 동기화한다 (1-J-3).
			const r = await runWithDriveAuth(
				targetKind === "drive",
				(silent) => call(commands.syncNow(false), { ...src, silent }),
				{ onReconnected: (s) => (drive = s) },
			);
			const m = r.merged.reduce((a, d) => ({ added: a.added + d.report.added, updated: a.updated + d.report.updated, deleted: a.deleted + d.report.deleted, both: a.both + d.report.conflicts + d.report.branches }), { added: 0, updated: 0, deleted: 0, both: 0 });
			if (r.merged.length) {
				toasts.success(
					$_("settings.syncSec.syncMerged", {
						values: {
							devices: r.merged.length,
							added: n(m.added),
							updated: n(m.updated),
							deleted: n(m.deleted),
							conflicts: m.both ? $_("settings.syncSec.syncConflictsPart", { values: { count: n(m.both) } }) : "",
						},
					}),
				);
			} else if (r.filesDown > 0 || r.filesUp > 0) {
				toasts.success($_("settings.syncSec.syncFilesMoved", { values: { down: n(r.filesDown), up: n(r.filesUp) } }));
			} else if (!r.filesMissing) {
				toasts.success($_("settings.syncSec.syncNoChanges"));
			}
			if (r.filesMissing > 0) {
				toasts.show(
					{
						tone: "warning",
						message: $_("settings.syncSec.syncMissingFilesWarn", { values: { count: n(r.filesMissing) } }),
						source: "sync",
					},
					7000,
				);
			}
			for (const d of r.skippedDevices ?? []) toasts.error($_("settings.syncSec.skippedDevice", { values: { device: label(d.deviceId), reason: d.reason } }));
			if (r.merged.some((d) => d.returnedAfterExclusion)) toasts.info($_("settings.syncSec.returnedExcluded"));
			await library.load();
			conflictsKey++;
		} catch (e) {
			if (e instanceof NativeDriveError) {
				toasts.error($_("settings.syncSec.drivePermError"), e.message);
			}
		} finally {
			syncing = false;
			await load();
		}
	}
	/** 기기 이름, 없으면 id 앞자리 (5-E-16-1) */
	const label = (id: string, name?: string | null) => deviceLabel(id, status?.peers ?? [], name);
	/** 이 기기 이름 (5-E-16). 다른 기기에는 다음 동기화부터 보인다 */
	async function renameDevice(name: string) {
		if (!status || name.trim() === (status.deviceName ?? "")) return;
		const saved = await call(commands.setDeviceName(name), src).catch(() => undefined);
		if (saved !== undefined && status) status = { ...status, deviceName: saved };
	}
	const n = (v: number) => v.toLocaleString();
	function size(b: number) {
		return b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`;
	}

	async function exportBundle() {
		const d = new Date();
		const stamp = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}-${String(d.getHours()).padStart(2, "0")}${String(d.getMinutes()).padStart(2, "0")}`;
		try {
			const r = await saveTo(`sandcastle-sync-${stamp}.scsync`, [SYNC_FILES], (path) => {
				busy = "export";
				return jobs.run($_("settings.syncSec.exportFileJob"), () => call(commands.exportSyncBundle(path, includeFiles), src), { group: $_("settings.syncSec.exportFileGroup") });
			});
			if (r) toasts.success($_("settings.syncSec.exportedToast", { values: { rows: n(r.rows), blobs: n(r.blobs), files: n(r.files), size: size(r.bytes) } }));
		} finally {
			busy = null;
		}
	}

	async function openBundle() {
		const [path] = await pickFiles([SYNC_FILES], false);
		if (path) await openBundleAt(path);
	}
	// 끌어다 놓아도 미리보기부터 (합치기는 그 뒤 버튼). 큰 묶음은 버튼으로
	$effect(() =>
		dropPaths({
			label: $_("settings.syncSec.dropOpenLabel"),
			hint: $_("settings.syncSec.dropOpenHint"),
			accept: (f) => extOf(f) === "scsync",
			multiple: false,
			maxBytes: 300 * 1024 * 1024,
			onpaths: ([path]) => openBundleAt(path),
		}),
	);
	async function openBundleAt(path: string) {
		busy = "open";
		try {
			pending = { path, preview: await call(commands.previewSyncBundle(path), src) };
		} catch {
			pending = null; // 이유는 call이 토스트로
		} finally {
			busy = null;
		}
	}

	async function apply() {
		if (!pending) return;
		busy = "apply";
		try {
			const path = pending.path;
			const r = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.applySyncBundle(path), src), { group: $_("settings.syncSec.applyGroup") });
			if (!r) return;
			const m = r.report;
			const parts = [
				m.added && `${$_("settings.syncSec.statAdded")} ${n(m.added)}`,
				m.updated && `${$_("settings.syncSec.statUpdated")} ${n(m.updated)}`,
				m.deleted && `${$_("settings.syncSec.statDeleted")} ${n(m.deleted)}`,
				m.restored && `${$_("settings.syncSec.statRestored")} ${n(m.restored)}`,
				r.files.added && `${$_("settings.syncSec.statFilesAdded")} ${n(r.files.added)}`,
				m.conflicts + m.branches && `${$_("settings.syncSec.statConflicts")} ${n(m.conflicts + m.branches)}`,
			].filter(Boolean);
			toasts.success(parts.length ? $_("settings.syncSec.mergedToast", { values: { parts: parts.join(" · ") } }) : $_("settings.syncSec.mergedNoChanges"));
			pending = null;
			conflictsKey++;
			await Promise.all([load(), library.load()]);
		} finally {
			busy = null;
		}
	}

	const rows = $derived.by(() => {
		if (!pending) return [];
		const m = pending.preview.report;
		const f = pending.preview.files;
		return [
			{ key: "added", label: $_("settings.syncSec.statAdded"), value: m.added, hint: $_("settings.syncSec.statAddedHint") },
			{ key: "updated", label: $_("settings.syncSec.statUpdated"), value: m.updated, hint: $_("settings.syncSec.statUpdatedHint") },
			{ key: "deleted", label: $_("settings.syncSec.statDeleted"), value: m.deleted, hint: $_("settings.syncSec.statDeletedHint") },
			{ key: "restored", label: $_("settings.syncSec.statRestored"), value: m.restored, hint: $_("settings.syncSec.statRestoredHint") },
			{ key: "keptLocal", label: $_("settings.syncSec.statKeptLocal"), value: m.keptLocalEdits, hint: $_("settings.syncSec.statKeptLocalHint") },
			{ key: "conflicts", label: $_("settings.syncSec.statConflicts"), value: m.conflicts, hint: $_("settings.syncSec.statConflictsHint") },
			{ key: "branches", label: $_("settings.syncSec.statBranches"), value: m.branches, hint: $_("settings.syncSec.statBranchesHint") },
			{ key: "filesAdded", label: $_("settings.syncSec.statFilesAdded"), value: f.added, hint: $_("settings.syncSec.statFilesAddedHint") },
			{ key: "filesMissing", label: $_("settings.syncSec.statFilesMissing"), value: f.missing, hint: $_("settings.syncSec.statFilesMissingHint") },
			{ key: "filesKeptLocal", label: $_("settings.syncSec.statFilesKeptLocal"), value: f.keptLocal, hint: $_("settings.syncSec.statFilesKeptLocalHint") },
		].filter((r) => r.value > 0);
	});
</script>

<Section title={$_("settings.syncSec.title")} lead={$_("settings.syncSec.lead")}>
	{#if !status}
		<div class="loading"><Spinner /></div>
	{:else}
		{#if status.pendingRollback}
			<div class="banner" role="status">
				<RotateCcw />
				<div><b>{$_("settings.syncSec.pendingRollbackTitle")}</b><span>{$_("settings.syncSec.pendingRollbackDesc")}</span></div>
				<Button size="sm" variant="ghost" onclick={cancelRollback}>{$_("common.cancel")}</Button>
				<Button size="sm" variant="primary" onclick={() => call(commands.restartApp(), src)}>{$_("settings.syncSec.restartBtn")}</Button>
			</div>
		{/if}
		{#key conflictsKey}<SyncConflicts onchange={load} peers={status.peers} />{/key}

		{#if showAuto}
			<Group title={$_("settings.syncSec.autoGroup")}>
				{#if layout.desktop}<Row label={$_("settings.syncSec.storage")} desc={$_("settings.syncSec.storageDesc")}>
					<Segmented
						label={$_("settings.syncSec.storageAria")}
						value={targetKind}
						options={[
							{ value: "folder", label: $_("settings.syncSec.folder") },
							{ value: "drive", label: $_("settings.syncSec.googleDrive") },
						]}
						onchange={(v) => savePart("sync", { target: v, auto: false })}
					/>
				</Row>{/if}
				{#if targetKind === "folder"}
					<Row label={$_("settings.syncSec.syncFolder")} desc={sync?.folder ?? $_("settings.syncSec.syncFolderDesc")}>
						{#if sync?.folder}<Button size="sm" variant="ghost" onclick={() => savePart("sync", { folder: null, auto: false })}>{$_("settings.syncSec.removeFolder")}</Button>{/if}
						<Button size="sm" onclick={chooseFolder}>{#snippet icon()}<FolderOpen />{/snippet}{sync?.folder ? $_("settings.syncSec.changeFolder") : $_("settings.syncSec.unsetFolder")}</Button>
					</Row>
				{:else if drive && !drive.available}
					<p class="meta empty">{$_("settings.syncSec.driveUnavailable")}</p>
				{:else if drive?.connected}
					<Row label={$_("settings.syncSec.googleDrive")} desc={wiping ? step || $_("settings.syncSec.preparing") : $_("settings.syncSec.driveConnectedDesc", { values: { who: drive.email ?? $_("settings.syncSec.connectedFallback") } })}>
						<Button size="sm" variant="ghost" loading={wiping} disabled={wiping || syncing} onclick={wipeDrive}>{$_("settings.syncSec.driveWipeBtn")}</Button>
						<Button size="sm" disabled={wiping || syncing} onclick={logoutDrive}>{#snippet icon()}<LogOut />{/snippet}{$_("settings.syncSec.driveLogoutBtn")}</Button>
					</Row>
				{:else}
					<Row label={$_("settings.syncSec.googleDrive")} desc={phone ? $_("settings.syncSec.driveLoginDescPhone") : $_("settings.syncSec.driveLoginDescDesktop")}>
						<Button size="sm" variant="primary" loading={loggingIn} disabled={loggingIn} onclick={loginDrive}>{#snippet icon()}<Cloud />{/snippet}{$_("settings.syncSec.loginBtn")}</Button>
					</Row>
				{/if}
				{#if sync && ready}
					<Row label={$_("settings.syncSec.syncNowBtn")} desc={syncing ? step || $_("settings.syncSec.preparing") : $_("settings.syncSec.syncNowDesc")}>
						<Button size="sm" variant="primary" loading={syncing} disabled={syncing || wiping} onclick={syncNow}>{#snippet icon()}<RefreshCw />{/snippet}{$_("settings.syncSec.syncNowBtn")}</Button>
					</Row>
					<Row label={$_("settings.syncSec.autoSync")} desc={$_("settings.syncSec.autoSyncDesc")} nowrap>
						{#if sync.auto}
							<Select value={String(sync.intervalMinutes)} options={INTERVALS} aria-label={$_("settings.syncSec.autoSyncIntervalAria")} onchange={(e) => savePart("sync", { intervalMinutes: Number(e.currentTarget.value) })} />
						{/if}
						<Switch checked={sync.auto} label={$_("settings.syncSec.autoSync")} hideLabel onchange={(v) => savePart("sync", { auto: v })} />
					</Row>
					{#if phone}
						<Row label={$_("settings.syncSec.wifiOnly")} desc={$_("settings.syncSec.wifiOnlyDesc")} nowrap>
							<Switch checked={sync.wifiOnly} label={$_("settings.syncSec.wifiOnly")} hideLabel onchange={(v) => savePart("sync", { wifiOnly: v })} />
						</Row>
					{/if}
				{/if}
			</Group>
		{/if}

		<Group title={$_("settings.syncSec.exportGroup")}>
			<Row label={$_("settings.syncSec.includeFiles")} desc={$_("settings.syncSec.includeFilesDesc")} nowrap>
				<Switch checked={includeFiles} label={$_("settings.syncSec.includeFiles")} hideLabel onchange={(v) => (includeFiles = v)} />
			</Row>
			<Row label={$_("settings.syncSec.createBundle")} desc={$_("settings.syncSec.createBundleDesc")} nowrap>
				<Button size="sm" loading={busy === "export"} disabled={busy !== null} onclick={exportBundle}>{#snippet icon()}<Download />{/snippet}{$_("settings.syncSec.createBtn")}</Button>
			</Row>
		</Group>

		<Group title={$_("settings.syncSec.importGroup")}>
			<Row label={$_("settings.syncSec.openBundle")} desc={$_("settings.syncSec.openBundleDesc")} nowrap>
				<Button size="sm" loading={busy === "open"} disabled={busy !== null} onclick={openBundle}>{#snippet icon()}<FolderOpen />{/snippet}{$_("settings.syncSec.openBtn")}</Button>
			</Row>
			{#if pending}
				{@const b = pending.preview.bundle}
				<div class="preview" role="region" aria-label={$_("settings.syncSec.previewAria")}>
					<p class="from">
						<Badge>{$_("settings.syncSec.deviceBadge", { values: { name: label(b.deviceId, b.deviceName) } })}</Badge>
						<span title={absolute(b.createdAt)}>{$_("settings.syncSec.createdWhen", { values: { when: relative(b.createdAt) } })}</span>
						<span class="meta">{$_("settings.syncSec.appVersionMeta", { values: { version: b.appVersion, size: size(b.bytes) } })}</span>
					</p>
					{#if rows.length}
						<dl class="counts">
							{#each rows as r (r.key)}
								<div title={r.hint}><dt>{r.label}</dt><dd>{n(r.value)}</dd></div>
							{/each}
						</dl>
					{:else}
						<p class="meta">{$_("settings.syncSec.nothingToChange")}</p>
					{/if}
					{#if pending.preview.report.settingsApplied}
						<p class="note"><Info /> {$_("settings.syncSec.settingsAppliedNote")}</p>
					{/if}
					{#if pending.preview.report.conflicts + pending.preview.report.branches > 0}
						<p class="note"><Info /> {$_("settings.syncSec.conflictsNote")}</p>
					{/if}
					<div class="acts">
						<Button size="sm" variant="ghost" disabled={busy !== null} onclick={() => (pending = null)}>{$_("common.cancel")}</Button>
						<Button size="sm" variant="primary" loading={busy === "apply"} disabled={busy !== null} onclick={apply}>{$_("settings.syncSec.mergeBtn")}</Button>
					</div>
				</div>
			{/if}
		</Group>

		<Group title={$_("settings.syncSec.devicesGroup")}>
			<Row label={$_("settings.syncSec.thisDevice")}>
				<Input
					value={status.deviceName ?? ""}
					placeholder={short(status.deviceId)}
					maxlength={40}
					aria-label={$_("settings.syncSec.thisDeviceAria")}
					onchange={(e) => void renameDevice(e.currentTarget.value)}
				/>
			</Row>
			{#if status.peers.length}
				<ul class="peers">
					{#each status.peers as p (p.deviceId)}
						<li>
							{#if p.name}<b title={p.deviceId}>{p.name}</b>{:else}<code>{short(p.deviceId)}</code>{/if}
							{#if p.excluded}<Badge>{$_("settings.syncSec.excludedBadge")}</Badge>{:else if p.stale}<Badge tone="warning">{$_("settings.syncSec.staleBadge")}</Badge>{/if}
							<span title={absolute(p.lastMergeAt)}>{$_("settings.syncSec.receivedFromPeer", { values: { when: relative(p.lastMergeAt) } })}</span>
							<span class="meta" title={p.theirAck ? absolute(p.theirAck) : undefined}>{p.theirAck ? $_("settings.syncSec.peerAcked", { values: { when: relative(p.theirAck) } }) : $_("settings.syncSec.peerNotAcked")} · {$_("settings.syncSec.mergeCount", { values: { count: n(p.merges) } })}</span>
							{#if p.excluded}
								<Button size="sm" variant="ghost" onclick={() => setExcluded(p.deviceId, false)}>{$_("settings.syncSec.reincludeBtn")}</Button>
							{:else if p.stale}
								<Button size="sm" variant="ghost" onclick={() => excludePeer(p.deviceId)}>{$_("settings.syncSec.excludeBtn")}</Button>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<p class="meta empty">{$_("settings.syncSec.noPeers")}</p>
			{/if}
		</Group>

		{#if backups.length}
			<Group title={$_("settings.syncSec.rollbackGroup")}>
				<p class="meta empty">{$_("settings.syncSec.rollbackGroupDesc")}</p>
				<ul class="peers">
					{#each backups as b (b.name)}
						<li>
							<span title={b.name}>{absolute(b.createdAt)}</span>
							<span class="meta">{relative(b.createdAt)} · {size(b.bytes)}</span>
							<span class="grow"></span>
							<Button size="sm" variant="ghost" onclick={() => rollback(b)}>{#snippet icon()}<RotateCcw />{/snippet}{$_("settings.syncSec.useBackupBtn")}</Button>
						</li>
					{/each}
				</ul>
			</Group>
		{/if}
	{/if}
</Section>

<style>
	.loading {
		display: flex;
		justify-content: center;
		padding: var(--space-6);
	}
	.preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin: var(--space-2) 0 var(--space-3);
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2, var(--surface));
	}
	.from {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		margin: 0;
	}
	.counts {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
		gap: var(--space-2);
		margin: 0;
	}
	.counts div {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-size: var(--text-sm);
	}
	.counts dt {
		color: var(--text-muted);
	}
	.counts dd {
		margin: 0;
		font-weight: 600;
	}
	.note {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.note :global(svg) {
		flex-shrink: 0;
		width: 1em;
		height: 1em;
		margin-top: 0.2em;
	}
	.acts {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.empty {
		margin: 0;
		padding: var(--space-3) 0;
	}
	.banner {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		margin-bottom: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2, var(--surface));
		flex-wrap: wrap;
	}
	.banner div {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 10rem;
	}
	.banner span {
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.grow {
		flex: 1;
	}
	.peers {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.peers li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) 0;
		border-top: 1px solid var(--border);
		font-size: var(--text-sm);
		flex-wrap: wrap;
	}
</style>
