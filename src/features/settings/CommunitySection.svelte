<script lang="ts">
	// 커뮤니티 플러그인 관리 (CLAUDE.md 6-D-8, ADR-012): 설치(미리보기) · 켜기(권한 동의) · 방식 · 설정 · 기록 · 업데이트 · 삭제
	import { onMount, untrack } from "svelte";
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

	const ZIP: Filter = { name: "플러그인 묶음", extensions: ["zip"] };

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
		const path = kind === "zip" ? ((await pickFiles([ZIP], false))[0] ?? null) : await pickFolder();
		if (!path) return;
		const preview = await admin.preview(path);
		if (!preview) return;
		if (updateId && preview.manifest.id !== updateId) {
			toasts.error(`다른 플러그인입니다 (${preview.manifest.id}) — '${updateId}'의 업데이트가 아닙니다`);
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
			toasts.success(was ? (p.enabled ? "업데이트했습니다" : "업데이트했습니다 (꺼져 있음 — 켜서 권한에 동의하세요)") : "설치했습니다 (꺼져 있음 — 아래 스위치로 켜세요)");
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
			title: `'${p.manifest?.name ?? p.id}' 삭제`,
			message: "휴지통으로 옮깁니다. 되살리면 꺼진 채로 돌아옵니다 (저장한 값도 같이).",
			confirmLabel: "삭제",
			danger: true,
		});
		if (ok) await admin.uninstall(p.id);
	}

	async function useVersion(p: UserPlugin, dir: string) {
		if (dir === p.activeDir) return;
		const r = await admin.switchVersion(p.id, dir);
		rev++;
		if (r) toasts.success(`${r.versions.find((v) => v.dir === dir)?.version ?? dir} 판으로 바꿨습니다 (꺼져 있음 — 켜서 권한에 동의하세요)`);
	}

	/** 업데이트로 권한이 늘어 꺼진 플러그인: 이전에 허락한 것이 있는데 지금 manifest가 더 요구함 */
	const needsConsent = (p: UserPlugin) => !p.enabled && p.granted.length > 0 && !!p.manifest && p.manifest.permissions.some((x) => !p.granted.includes(x));
	const has = (p: UserPlugin, perm: string) => !!p.manifest?.permissions.includes(perm);
</script>

<Section title="커뮤니티 플러그인" lead="남이 만든 플러그인을 파일로 설치합니다. 격리된 곳에서 허락한 권한만으로 동작하고, 설치해도 꺼진 채로 시작합니다. 믿을 수 있는 곳에서 받은 것만 설치하세요">
	<Group>
		<Row label="커뮤니티 플러그인 사용" desc={on ? "" : "꺼져 있는 동안은 설치된 플러그인을 읽지도 실행하지도 않습니다"}>
			<Switch checked={on} label="커뮤니티 플러그인 사용" hideLabel onchange={(v) => plugins.set("community", v)} />
		</Row>
	</Group>
	{#if on}
	<Group title="설치">
		<Row label="zip으로 설치" desc="manifest.json이 든 zip 파일. 설치 전에 내용과 권한을 미리 봅니다">
			<Button size="sm" onclick={() => pick("zip")}>{#snippet icon()}<FileArchive />{/snippet}zip 고르기</Button>
		</Row>
		{#if !layout.mobile}
			<Row label="폴더에서 설치" desc="만들고 있는 플러그인 폴더를 그대로">
				<Button size="sm" onclick={() => pick("folder")}>{#snippet icon()}<FolderOpen />{/snippet}폴더 고르기</Button>
			</Row>
		{/if}
		<Row label="플러그인의 고치기 제안" desc="플러그인이 캐릭터·로어북 등을 고치자고 한 것. 바뀌는 곳을 보고 승인해야만 바뀝니다">
			<Button size="sm" variant="ghost" onclick={() => (proposalsUi.open = true)}>보기{#if proposals.pending.length > 0}&nbsp;<Badge tone="accent">{proposals.pending.length}</Badge>{/if}</Button>
		</Row>
	</Group>

	<Group title={`설치된 플러그인${admin.list.length ? ` (${admin.list.length})` : ""}`}>
		{#if admin.loaded && admin.list.length === 0}
			<EmptyState title="설치된 플러그인이 없습니다" description="zip이나 폴더를 골라 설치하세요. 플러그인 만드는 법은 docs의 '플러그인 만들기'에 있습니다" />
		{/if}
		{#each admin.list as p (p.id)}
			{@const m = p.manifest}
			<div class="plugin">
				<Row label={m?.name ?? p.id} desc={m ? `${m.description || "설명 없음"} · v${m.version}${m.author ? ` · ${m.author}` : ""} · ${p.id}` : `읽을 수 없는 플러그인 · ${p.id}`}>
					{#if p.error}<Badge tone="danger">오류</Badge>{/if}
					{#if p.codeChanged}<Badge tone="danger">코드가 바뀌어 꺼짐</Badge>{/if}
					{#if needsConsent(p)}<Badge tone="warning">권한이 늘어 꺼짐</Badge>{/if}
					{#if p.enabled && p.processMode === "fix"}<Badge tone="accent">자동 고침</Badge>{/if}
					{#if p.enabled && p.aiAuto}<Badge tone="warning">자동 AI</Badge>{/if}
					<IconButton label={expanded === p.id ? "접기" : "자세히"} size="sm" onclick={() => (expanded = expanded === p.id ? null : p.id)}><ChevronDown class={expanded === p.id ? "open" : ""} /></IconButton>
					{#key `${p.id}:${p.enabled}:${rev}`}
						<Switch checked={p.enabled} label="{m?.name ?? p.id} 켜기" hideLabel disabled={!m} onchange={(v) => toggle(p, v)} />
					{/key}
				</Row>

				{#if expanded === p.id}
					<div class="detail">
						{#if p.error}
							<p class="err">{p.error}</p>
						{/if}
						{#if p.codeChanged}
							<p class="err">켤 때 본 코드와 폴더의 파일이 달라져서 꺼 두었습니다 (직접 고쳤거나 다른 곳에서 바뀜). 믿을 수 있는 변경인지 확인하고, 괜찮으면 다시 켜세요.</p>
						{/if}
						{#if p.versions.length > 1}
							<h4>사용할 판</h4>
							<Select
								value={p.activeDir}
								aria-label="사용할 판"
								options={p.versions.map((v) => ({ value: v.dir, label: `v${v.version}${v.dir === p.activeDir ? " (사용 중)" : ""}${v.error ? " — 읽을 수 없음" : ""}`, disabled: !!v.error }))}
								onchange={(e) => useVersion(p, e.currentTarget.value)}
							/>
						{/if}
						{#if m}
							<h4>권한</h4>
							<PermissionList perms={m.permissions} added={needsConsent(p) ? m.permissions.filter((x) => !p.granted.includes(x)) : []} />

							{#if has(p, "messages.process")}
								<h4>AI 응답 후처리 방식</h4>
								<Segmented
									value={(p.processMode === "fix" ? "fix" : "notes") as "notes" | "fix"}
									label="후처리 방식"
									options={[
										{ value: "notes", label: "표시만 (기본)" },
										{ value: "fix", label: "자동 고침" },
									]}
									onchange={(v) => admin.setProcessMode(p.id, v)}
								/>
								<p class="hint">표시만: 메모만 응답 아래에 붙입니다. 자동 고침: 응답 글을 바꿉니다 (원문은 남고 '원문 보기·되돌리기'가 있음)</p>
							{/if}

							{#if has(p, "ai")}
								<h4>AI 호출</h4>
								{#key p.aiAuto}
								<Switch
									checked={p.aiAuto}
									label="자동 AI 호출 허용"
									description="끄면 이 플러그인은 사용자가 누른 명령 안에서만 AI를 부를 수 있고, 패널에서는 부를 때마다 확인을 묻습니다. 켜면 응답 후처리·프롬프트 훅처럼 사용자가 누르지 않은 곳에서도 부르며 요금이 들 수 있습니다 (예산 한도 안에서)"
									onchange={(v) => admin.setAiAuto(p.id, v)}
								/>
								{/key}
							{/if}

							{#if m.settings.length}
								<h4>설정</h4>
								<SettingsForm pluginId={p.id} fields={m.settings} />
							{/if}
						{/if}
						<div class="actions">
							<Button size="sm" variant="ghost" onclick={() => ((logFor = p), (logOpen = true))}>기록 보기</Button>
							<Button size="sm" variant="ghost" onclick={() => pick("zip", p.id)}>zip으로 업데이트</Button>
							{#if !layout.mobile}<Button size="sm" variant="ghost" onclick={() => pick("folder", p.id)}>폴더로 업데이트</Button>{/if}
							<Button size="sm" variant="danger" onclick={() => remove(p)}>삭제</Button>
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
