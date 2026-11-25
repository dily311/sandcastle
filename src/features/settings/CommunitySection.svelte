<script lang="ts">
	// 커뮤니티 플러그인 관리 (CLAUDE.md 6-D-8, ADR-012): 설치(미리보기) · 켜기(권한 동의) · 방식 · 설정 · 기록 · 업데이트 · 삭제
	import { onMount, untrack } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronDown, FileArchive, FolderOpen, Puzzle } from "@lucide/svelte";
	import type { PluginPreview, UserPlugin_Serialize as UserPlugin } from "$ipc/bindings";
	import { Badge, Button, EmptyState, IconButton, Segmented, Select, Switch } from "$shared/ui";
	import { pickFiles, pickFolder, type Filter } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { layout } from "$app/layout.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { admin } from "$features/community/admin.svelte";
	import { proposals, proposalsUi } from "$features/community/proposals.svelte";
	import ConsentDialog from "$features/community/ConsentDialog.svelte";
	import InstallDialog from "$features/community/InstallDialog.svelte";
	import LogDialog from "$features/community/LogDialog.svelte";
	import PermissionList from "$features/community/PermissionList.svelte";
	import SettingsForm from "$features/community/SettingsForm.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const zipFilter = (): Filter => ({ name: $_("settings.communitySec.zipFilter"), extensions: ["zip"] });

	// 화면 상태 (데이터는 admin 저장소)
	let expanded = $state<string | null>(null);
	let installing = $state<{ path: string; preview: PluginPreview } | null>(null);
	let installOpen = $state(false);
	let consentFor = $state<UserPlugin | null>(null);
	let consentOpen = $state(false);
	let logFor = $state<UserPlugin | null>(null);
	let logOpen = $state(false);
	let busy = $state(false);
	/** Switch는 누르면 스스로 켜진 모양이 된다. 동의 창을 취소하면 꺼진 채여야 하므로 닫힐 때마다 다시 그린다 */
	let rev = $state(0);
	$effect(() => {
		if (!consentOpen) untrack(() => rev++);
	});

	onMount(() => void plugins.load());
	const on = $derived(plugins.on("community"));
	// 코어 플러그인이 꺼져 있으면 백엔드가 거절하므로 켜진 동안에만 불러오고 지켜봄
	$effect(() => {
		if (!on) return;
		let stop: (() => void) | undefined;
		let gone = false;
		void admin.watch().then((s) => (gone ? s() : (stop = s)));
		void proposals.watch();
		return () => {
			gone = true;
			stop?.();
		};
	});

	/** 묶음을 고르고 미리보기. updateId가 있으면 그 플러그인의 업데이트여야 한다 */
	async function pick(kind: "zip" | "folder", updateId?: string) {
		const path = kind === "zip" ? ((await pickFiles([zipFilter()], false))[0] ?? null) : await pickFolder();
		if (!path) return;
		const preview = await admin.preview(path);
		if (!preview) return;
		if (updateId && preview.manifest.id !== updateId) {
			toasts.error($_("settings.communitySec.wrongUpdateId", { values: { found: preview.manifest.id, expected: updateId } }));
			return;
		}
		installing = { path, preview };
		installOpen = true;
	}

	async function install() {
		if (!installing) return;
		busy = true;
		try {
			const p = await admin.install(installing.path, installing.preview.hash);
			if (!p) return;
			installOpen = false;
			expanded = p.id;
			await admin.load();
			const was = installing.preview.existing;
			toasts.success(was ? (p.enabled ? $_("settings.communitySec.updatedEnabled") : $_("settings.communitySec.updatedDisabled")) : $_("settings.communitySec.installedDisabled"));
		} finally {
			busy = false;
		}
	}

	function toggle(p: UserPlugin, on: boolean) {
		if (!on) return void admin.disable(p.id);
		consentFor = p;
		consentOpen = true;
	}
	async function confirmEnable() {
		if (!consentFor?.manifest) return;
		busy = true;
		try {
			if (await admin.enable(consentFor.id, consentFor.manifest.permissions)) consentOpen = false;
		} finally {
			busy = false;
		}
	}

	async function remove(p: UserPlugin) {
		const ok = await confirmAction({
			title: $_("settings.communitySec.deleteTitle", { values: { name: p.manifest?.name ?? p.id } }),
			message: $_("settings.communitySec.deleteMsg"),
			confirmLabel: $_("common.delete"),
			danger: true,
		});
		if (ok) await admin.uninstall(p.id);
	}

	async function useVersion(p: UserPlugin, dir: string) {
		if (dir === p.activeDir) return;
		const r = await admin.switchVersion(p.id, dir);
		rev++;
		if (r) toasts.success($_("settings.communitySec.switchedVersion", { values: { version: r.versions.find((v) => v.dir === dir)?.version ?? dir } }));
	}

	/** 업데이트로 권한이 늘어 꺼진 플러그인: 이전에 허락한 것이 있는데 지금 manifest가 더 요구함 */
	const needsConsent = (p: UserPlugin) => !p.enabled && p.granted.length > 0 && !!p.manifest && p.manifest.permissions.some((x) => !p.granted.includes(x));
	const has = (p: UserPlugin, perm: string) => !!p.manifest?.permissions.includes(perm);
</script>

<Section title={$_("settings.communitySec.title")} lead={$_("settings.communitySec.lead")}>
	<Group>
		<Row label={$_("settings.communitySec.title")}>
			<Switch checked={on} label={$_("settings.communitySec.title")} hideLabel onchange={(v) => plugins.set("community", v)} />
		</Row>
	</Group>
	{#if on}
	<Group title={$_("settings.communitySec.installGroup")}>
		<Row label={$_("settings.communitySec.installZip")} desc={$_("settings.communitySec.installZipDesc")}>
			<Button size="sm" onclick={() => pick("zip")}>{#snippet icon()}<FileArchive />{/snippet}{$_("settings.communitySec.pickZip")}</Button>
		</Row>
		{#if !layout.mobile}
			<Row label={$_("settings.communitySec.installFolder")} desc={$_("settings.communitySec.installFolderDesc")}>
				<Button size="sm" onclick={() => pick("folder")}>{#snippet icon()}<FolderOpen />{/snippet}{$_("settings.communitySec.pickFolder")}</Button>
			</Row>
		{/if}
		<Row label={$_("settings.communitySec.proposals")} desc={$_("settings.communitySec.proposalsDesc")}>
			<Button size="sm" variant="ghost" onclick={() => (proposalsUi.open = true)}>{$_("settings.communitySec.viewBtn")}{#if proposals.pending.length > 0}&nbsp;<Badge tone="accent">{proposals.pending.length}</Badge>{/if}</Button>
		</Row>
	</Group>

	<Group title={admin.list.length ? $_("settings.communitySec.installedGroupCount", { values: { count: admin.list.length } }) : $_("settings.communitySec.installedGroup")}>
		{#if admin.loaded && admin.list.length === 0}
			<EmptyState title={$_("settings.communitySec.emptyTitle")} description={$_("settings.communitySec.emptyDesc")} />
		{/if}
		{#each admin.list as p (p.id)}
			{@const m = p.manifest}
			<div class="plugin">
				<Row label={m?.name ?? p.id} desc={m ? `${m.description || $_("settings.communitySec.noDesc")} · v${m.version}${m.author ? ` · ${m.author}` : ""} · ${p.id}` : $_("settings.communitySec.unreadable", { values: { id: p.id } })}>
					{#if p.error}<Badge tone="danger">{$_("settings.communitySec.errBadge")}</Badge>{/if}
					{#if p.codeChanged}<Badge tone="danger">{$_("settings.communitySec.codeChangedBadge")}</Badge>{/if}
					{#if needsConsent(p)}<Badge tone="warning">{$_("settings.communitySec.needsConsentBadge")}</Badge>{/if}
					{#if p.enabled && p.processMode === "fix"}<Badge tone="accent">{$_("settings.communitySec.autoFixBadge")}</Badge>{/if}
					{#if p.enabled && p.aiAuto}<Badge tone="warning">{$_("settings.communitySec.autoAiBadge")}</Badge>{/if}
					<IconButton label={expanded === p.id ? $_("settings.communitySec.collapse") : $_("settings.communitySec.details")} size="sm" onclick={() => (expanded = expanded === p.id ? null : p.id)}><ChevronDown class={expanded === p.id ? "open" : ""} /></IconButton>
					{#key `${p.id}:${p.enabled}:${rev}`}
						<Switch checked={p.enabled} label={$_("settings.communitySec.enableAria", { values: { name: m?.name ?? p.id } })} hideLabel disabled={!m} onchange={(v) => toggle(p, v)} />
					{/key}
				</Row>

				{#if expanded === p.id}
					<div class="detail">
						{#if p.error}
							<p class="err">{p.error}</p>
						{/if}
						{#if p.codeChanged}
							<p class="err">{$_("settings.communitySec.codeChangedMsg")}</p>
						{/if}
						{#if p.versions.length > 1}
							<h4>{$_("settings.communitySec.activeVersion")}</h4>
							<Select
								value={p.activeDir}
								aria-label={$_("settings.communitySec.activeVersion")}
								options={p.versions.map((v) => ({ value: v.dir, label: `v${v.version}${v.dir === p.activeDir ? $_("settings.communitySec.inUseTag") : ""}${v.error ? $_("settings.communitySec.unreadableTag") : ""}`, disabled: !!v.error }))}
								onchange={(e) => useVersion(p, e.currentTarget.value)}
							/>
						{/if}
						{#if m}
							<h4>{$_("settings.communitySec.permissions")}</h4>
							<PermissionList perms={m.permissions} added={needsConsent(p) ? m.permissions.filter((x) => !p.granted.includes(x)) : []} />

							{#if has(p, "messages.process")}
								<h4>{$_("settings.communitySec.processModeHeading")}</h4>
								<Segmented
									value={(p.processMode === "fix" ? "fix" : "notes") as "notes" | "fix"}
									label={$_("settings.communitySec.processModeAria")}
									options={[
										{ value: "notes", label: $_("settings.communitySec.modeNotes") },
										{ value: "fix", label: $_("settings.communitySec.modeFix") },
									]}
									onchange={(v) => admin.setProcessMode(p.id, v)}
								/>
								<p class="hint">{$_("settings.communitySec.processModeHint")}</p>
							{/if}

							{#if has(p, "ai")}
								<h4>{$_("settings.communitySec.aiCallHeading")}</h4>
								{#key p.aiAuto}
								<Switch
									checked={p.aiAuto}
									label={$_("settings.communitySec.aiAutoLabel")}
									description={$_("settings.communitySec.aiAutoDesc")}
									onchange={(v) => admin.setAiAuto(p.id, v)}
								/>
								{/key}
							{/if}

							{#if m.settings.length}
								<h4>{$_("settings.communitySec.settingsHeading")}</h4>
								<SettingsForm pluginId={p.id} fields={m.settings} />
							{/if}
						{/if}
						<div class="actions">
							<Button size="sm" variant="ghost" onclick={() => ((logFor = p), (logOpen = true))}>{$_("settings.communitySec.viewLogs")}</Button>
							<Button size="sm" variant="ghost" onclick={() => pick("zip", p.id)}>{$_("settings.communitySec.updateZip")}</Button>
							{#if !layout.mobile}<Button size="sm" variant="ghost" onclick={() => pick("folder", p.id)}>{$_("settings.communitySec.updateFolder")}</Button>{/if}
							<Button size="sm" variant="danger" onclick={() => remove(p)}>{$_("common.delete")}</Button>
						</div>
					</div>
				{/if}
			</div>
		{/each}
	</Group>
	{/if}
</Section>

<InstallDialog bind:open={installOpen} target={installing} {busy} oninstall={install} />
<ConsentDialog bind:open={consentOpen} plugin={consentFor} {busy} onconfirm={confirmEnable} />
{#if logFor}<LogDialog bind:open={logOpen} pluginId={logFor.id} name={logFor.manifest?.name ?? logFor.id} />{/if}

<style>
	.plugin :global(.open) {
		transform: rotate(180deg);
	}
	.detail {
		padding: var(--space-2) var(--space-4) var(--space-4);
		display: grid;
		gap: var(--space-2);
	}
	h4 {
		margin: var(--space-2) 0 0;
		font-size: var(--text-md);
	}
	.hint {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.err {
		margin: 0;
		color: var(--danger);
		font-size: var(--text-sm);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
		margin-top: var(--space-2);
	}
</style>
