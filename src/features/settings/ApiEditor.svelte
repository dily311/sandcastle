<script lang="ts">
	// API 연결 추가·편집 (모달 대신 설정 안 페이지, 테마 편집과 같은 방식)
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { CircleAlert, CircleCheck, ChevronLeft, FileJson, KeyRound, Zap } from "@lucide/svelte";
	import { apiLists, firstModel, modelLabel } from "./providers";
	import { Badge, Button, Field, IconButton, Input, Select, Spinner } from "$shared/ui";
	import ModelPicker from "./ModelPicker.svelte";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library } from "$features/character/library.svelte";
	import { drops, extOf } from "$app/drop.svelte";

	/** 연결 id, 또는 "new" */
	let { editId }: { editId: string } = $props();

	const src = { source: "api" };
	const isNew = $derived(editId === "new");

	let loaded = $state(false);
	let id = $state<string | null>(null);
	let providerId = $state(apiLists[0].provider);
	let name = $state(apiLists[0].label);
	let baseUrl = $state(apiLists[0].baseUrl);
	let key = $state("");
	let hasKey = $state(false);
	let verified = $state(false);
	let saving = $state(false);
	let testing = $state(false);
	/** 연결 테스트에만 임시로 쓰는 모델 (프로필에 저장하지 않음) */
	let testModel = $state<string | null>(null);
	// 연결 테스트 결과 (보낸 "Hi" + 모델 답변)
	let check = $state<{ state: "checking" | "ok" | "fail"; message: string; model?: string; reply?: string; isChat?: boolean } | null>(null);

	// Vertex AI: 서비스 계정 JSON + 지역 + 프로젝트 → 백엔드가 받는 키 JSON으로 묶음
	let saFile = $state<{ name: string; text: string; email: string } | null>(null);
	let savedEmail = $state<string | null>(null);
	let location = $state("global");
	let projectId = $state("");
	const LOCATIONS = ["global", "us-central1", "us-east5", "europe-west1", "europe-west4", "asia-northeast1", "asia-northeast3"];

	let initial = $state("");
	const snapshot = () => JSON.stringify([providerId, name, baseUrl, key, saFile?.text ?? "", location, projectId]);
	const dirty = $derived(loaded && snapshot() !== initial);

	const provider = $derived(apiLists.find((p) => p.provider === providerId));
	const vertex = $derived(provider?.formatType === "vertexai");
	const isPluginProvider = $derived(provider?.formatType === "elevenlabs" || provider?.formatType === "novelai");
	const currentProfile = $derived(id ? library.apis.find((x) => x.id === id) : undefined);

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? library.refreshApis() : library.load()]);
		if (!isNew) {
			const a = library.apis.find((x) => x.id === editId);
			if (!a) return back(true);
			id = a.id;
			providerId = a.provider;
			name = a.name;
			baseUrl = a.baseUrl;
			verified = !!a.verified;
			const p = apiLists.find((x) => x.provider === a.provider);
			testModel = a.model || firstModel(p) || null;
			hasKey = await call(commands.hasApiKey(a.id), { ...src, silent: true }).catch(() => false);
			if (a.formatType === "vertexai" && hasKey) {
				const info = await call(commands.getVertexKeyInfo(a.id), { ...src, silent: true }).catch(() => null);
				if (info) {
					projectId = info.projectId;
					location = info.location || "global";
					savedEmail = info.clientEmail;
				}
			}
		}
		initial = snapshot();
		loaded = true;
	});
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: $_("settings.more3.unsavedTitle"), message: $_("settings.more3.discardApiMsg"), confirmLabel: $_("settings.more3.discardBtn") }));
	});

	function back(force = false) {
		if (force) nav.clearGuard();
		void nav.replace({ page: "settings", tab: "api" });
	}

	function changeProvider(next: string) {
		const p = apiLists.find((x) => x.provider === next);
		if (!p) return;
		const prev = provider;
		// 이름을 손대지 않았으면 공급자 이름을 따라감
		if (!name || name === prev?.label) name = p.label;
		providerId = p.provider;
		baseUrl = p.baseUrl;
	}

	let fileInput: HTMLInputElement | undefined = $state();
	function pickServiceAccount(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = "";
		if (file) void importServiceAccount(file);
	}
	// Vertex 연결일 때 키 파일을 끌어다 놓기. 비밀 파일이라 디스크에 쓰지 않고 메모리에서만 읽음
	$effect(() => {
		if (!vertex) return;
		return drops.register({
			label: $_("settings.more3.dropSaLabel"),
			hint: $_("settings.more3.dropSaHint"),
			accept: (f) => extOf(f) === "json",
			ondrop: ([file]) => importServiceAccount(file),
		});
	});
	async function importServiceAccount(file: File) {
		try {
			const text = await file.text();
			const j = JSON.parse(text);
			if (j.type !== "service_account" || !j.client_email || !j.private_key) throw new Error($_("settings.more3.invalidSaFile"));
			saFile = { name: file.name, text, email: j.client_email };
			if (j.project_id) projectId = j.project_id;
			toasts.success($_("settings.more3.saLoadedToast"));
		} catch (err) {
			toasts.error(err instanceof SyntaxError ? $_("settings.more3.invalidJson") : String((err as Error).message ?? err));
		}
	}

	/** 저장할 키: Vertex는 묶은 JSON, 아니면 붙여 넣은 키. 빈 문자열이면 키는 그대로 둠 */
	function credential(): string {
		if (!vertex) return key;
		if (!saFile) return "";
		return JSON.stringify({ service_account: saFile.text, location, project_id: projectId.trim() });
	}
	const vertexNeedsFile = $derived(vertex && !saFile && (!hasKey || isNew));
	const canSave = $derived(!vertex || (!vertexNeedsFile && (!saFile || !!projectId.trim())));

	async function test() {
		if (!id) return;
		const chosen = isPluginProvider ? null : testModel?.trim() || null;
		const isChat = !isPluginProvider && !!chosen;
		const modelDisplay = chosen ? modelLabel(provider, chosen) : "";
		testing = true;
		check = { state: "checking", message: $_("settings.more3.checkingMsg"), model: modelDisplay, isChat };
		try {
			const reply = await call(commands.testMessage(id, chosen), { ...src, silent: true });
			verified = true;
			const a = library.apis.find((x) => x.id === id);
			if (a) a.verified = true;
			check = {
				state: "ok",
				message: $_("settings.more.apiConnected", { values: { reply: reply.trim().slice(0, 60) || $_("settings.more.emptyReply") } }),
				model: modelDisplay,
				reply: reply.trim(),
				isChat,
			};
		} catch (e) {
			const msg = (e as { message?: string })?.message ?? String(e);
			check = { state: "fail", message: msg, model: modelDisplay, isChat };
		} finally {
			testing = false;
		}
	}

	async function save() {
		if (!provider || !canSave) return;
		saving = true;
		const input = { name: name.trim() || provider.label, provider: provider.provider, formatType: provider.formatType, baseUrl: baseUrl.trim(), model: "" };
		const cred = credential();
		try {
			if (id) {
				const a = library.apis.find((x) => x.id === id);
				const endpointChanged = a ? a.baseUrl.trim() !== input.baseUrl || a.formatType !== input.formatType : false;
				await call(commands.updateApiProfile(id, input, cred), src);
				if (cred || endpointChanged) verified = false;
				if (a) Object.assign(a, input, { updatedAt: Date.now(), verified });
				if (cred) {
					hasKey = true;
					if (saFile) savedEmail = saFile.email;
				}
			} else {
				const created = await call(commands.createApiProfile(input, cred), src);
				library.apis = [...library.apis, created];
				id = created.id;
				hasKey = !!cred;
				verified = false;
				testModel = firstModel(provider) || null;
				if (saFile) savedEmail = saFile.email;
				// 첫 연결이면 기본값으로
				if (!appSetting.value?.selectedApiProfile) await appSetting.update({ selectedApiProfile: created.id });
			}
			key = "";
			saFile = null;
			initial = snapshot();
			// 저장했으니 나가기 확인은 필요 없음 (가드 효과가 다시 돌기 전에 이동하므로 직접 지움)
			nav.clearGuard();
			if (isNew && id) void nav.replace({ page: "settings", tab: "api", edit: id });
			toasts.success($_("settings.more3.savedToast"));
		} finally {
			saving = false;
		}
	}
	async function removeKey() {
		if (!id) return;
		await call(commands.deleteApiKey(id), src);
		hasKey = false;
		savedEmail = null;
		projectId = "";
		toasts.success($_("settings.more3.keyDeletedToast"));
	}
</script>

{#if !loaded}
	<div class="center"><Spinner /></div>
{:else}
	<div class="crumb">
		<IconButton label={$_("settings.more3.backToApiAria")} size="sm" onclick={() => back()}><ChevronLeft /></IconButton>
		<span>{$_("settings.more3.apiKeysLabel")}</span>
	</div>
	<header class="head">
		<h1>{isNew ? $_("settings.more3.addConn") : name || $_("settings.more3.editConn")}</h1>
		<div class="btns">
			{#if dirty}<span class="dirty">{$_("settings.more3.dirty")}</span>{/if}
			<Button variant="ghost" onclick={() => back()}>{dirty ? $_("common.cancel") : $_("common.close")}</Button>
			<Button variant="primary" loading={saving} disabled={!canSave || (!!id && !dirty)} onclick={save}>{$_("common.save")}</Button>
		</div>
	</header>
	<p class="lead">{$_("settings.more3.apiKeyLead")}</p>

	<div class="card form">
		<div class="two">
			<Field label={$_("settings.more3.provider")}>
				<Select value={providerId} options={apiLists.map((p) => ({ value: p.provider, label: p.label }))} disabled={!!id} onchange={(e) => changeProvider(e.currentTarget.value)} />
			</Field>
			<Field label={$_("settings.more3.name")}><Input bind:value={name} placeholder={provider?.label} /></Field>
		</div>
	</div>

	{#if vertex}
		<h2>{$_("settings.more3.serviceAccount")}</h2>
		<div class="card form">
			<div class="sa">
				<span class="sa-icon"><FileJson /></span>
				<div class="sa-txt">
					{#if saFile}
						<b>{saFile.name}</b><small>{saFile.email}</small>
					{:else if hasKey}
						<b>{projectId || $_("settings.more3.savedSaExists")}</b>
						<small>{[location, savedEmail].filter(Boolean).join(" · ") || $_("settings.more3.saReloadHint")}</small>
					{:else}
						<b>{$_("settings.more3.noKeyFile")}</b><small>{$_("settings.more3.saConsoleHint")}</small>
					{/if}
				</div>
				{#if hasKey && !saFile}<Button variant="ghost" onclick={removeKey}>{$_("common.delete")}</Button>{/if}
				<Button variant={saFile || hasKey ? "secondary" : "primary"} onclick={() => fileInput?.click()}>{$_("settings.more3.loadJsonBtn")}</Button>
				<input bind:this={fileInput} type="file" accept=".json,application/json" hidden onchange={pickServiceAccount} />
			</div>
			<div class="two">
				<Field label={$_("settings.more3.locationLabel")} hint={hasKey && !saFile ? $_("settings.more3.saReloadHint") : $_("settings.more3.locationGlobalHint")}>
					<Select value={location} options={LOCATIONS.map((l) => ({ value: l, label: l }))} disabled={hasKey && !saFile} onchange={(e) => (location = e.currentTarget.value)} />
				</Field>
				<Field label={$_("settings.more3.projectIdLabel")} hint={hasKey && !saFile ? $_("settings.more3.projectIdSavedHint") : $_("settings.more3.projectIdAutoHint")}>
					<Input bind:value={projectId} placeholder="my-project-123" readonly={hasKey && !saFile} />
				</Field>
			</div>
		</div>
	{:else}
		<h2>{$_("settings.more3.authHeading")}</h2>
		<div class="card form">
			<Field label={$_("settings.more3.apiKeysLabel")} hint={id ? (hasKey ? $_("settings.more3.keySavedHint") : $_("settings.more3.keyMissingHint")) : undefined}>
				<div class="inline">
					<Input type="password" bind:value={key} placeholder={hasKey ? "••••••••" : $_("settings.more3.pasteKeyPlaceholder")} autocomplete="off">
						{#snippet leading()}<KeyRound />{/snippet}
					</Input>
					{#if hasKey}<Button variant="ghost" onclick={removeKey}>{$_("settings.more3.deleteKeyBtn")}</Button>{/if}
				</div>
			</Field>
			<Field label={$_("settings.more3.baseUrlLabel")} hint={$_("settings.more3.baseUrlHint")}>
				<Input bind:value={baseUrl} placeholder="https://" />
			</Field>
		</div>
	{/if}
	{#if id && !hasKey}<p class="warn"><Badge tone="warning"><KeyRound /> {$_("settings.apiSec.noKey")}</Badge> {$_("settings.more3.needKeyWarn")}</p>{/if}

	{#if id}
		<h2 class="test-head">
			{$_("settings.more3.connTestHeading")}
			{#if verified}
				<Badge tone="success">{$_("settings.more3.verified")}</Badge>
			{:else}
				<Badge tone="neutral">{$_("settings.more3.unverified")}</Badge>
			{/if}
		</h2>
		<div class="card form">
			<div class="test-bar">
				{#if !isPluginProvider}
					<Field label={$_("settings.more3.testModelLabel")} hint={$_("settings.more3.testModelHint")}>
						<div class="test-action">
							<ModelPicker profile={currentProfile} value={testModel} onchange={(m) => (testModel = m)} label={$_("settings.more3.testModelLabel")} />
							<Button
								loading={testing}
								disabled={dirty || !hasKey || (!isPluginProvider && !testModel)}
								title={dirty ? $_("settings.more3.saveBeforeTestTitle") : !isPluginProvider && !testModel ? $_("settings.more3.pickTestModelTitle") : undefined}
								onclick={test}
							>
								{#snippet icon()}<Zap />{/snippet}{$_("settings.more3.connTestHeading")}
							</Button>
						</div>
					</Field>
				{:else}
					<div class="test-action">
						<ModelPicker profile={currentProfile} value={testModel} onchange={(m) => (testModel = m)} label={$_("settings.more3.testModelLabel")} />
						<Button
							loading={testing}
							disabled={dirty || !hasKey || (!isPluginProvider && !testModel)}
							title={dirty ? $_("settings.more3.saveBeforeTestTitle") : !isPluginProvider && !testModel ? $_("settings.more3.pickTestModelTitle") : undefined}
							onclick={test}
						>
							{#snippet icon()}<Zap />{/snippet}{$_("settings.more3.connTestHeading")}
						</Button>
					</div>
				{/if}
			</div>
			{#if check}
				{#if check.isChat}
					<div class="test-chat" role="status">
						<div class="chat-line user">
							<span class="chat-role">{$_("settings.more3.sentMsgRole")}</span>
							<div class="chat-bubble">Hi</div>
						</div>
						<div class="chat-line assistant {check.state}">
							<span class="chat-role">
								{#if check.state === "checking"}<Spinner size={12} />{:else if check.state === "ok"}<CircleCheck />{:else}<CircleAlert />{/if}
								{check.model || $_("settings.more2.modelDefault")}
							</span>
							<div class="chat-bubble">
								{#if check.state === "checking"}
									{$_("settings.more3.waitingReply")}
								{:else if check.state === "ok"}
									{check.reply || $_("settings.more.emptyReply")}
								{:else}
									{check.message}
								{/if}
							</div>
						</div>
					</div>
				{:else}
					<div class="check {check.state}" role="status">
						{#if check.state === "checking"}<Spinner size={14} />{:else if check.state === "ok"}<CircleCheck />{:else}<CircleAlert />{/if}
						<span>{check.message}</span>
					</div>
				{/if}
			{/if}
		</div>
	{/if}
{/if}

<style>
	.test-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.test-bar {
		display: flex;
		align-items: flex-end;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.test-bar > :global(:first-child) {
		flex: 1;
		min-width: 200px;
	}
	.test-action {
		display: flex;
		gap: var(--space-3);
	}
	.test-chat {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--border);
	}
	.chat-line {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-width: 85%;
	}
	.chat-line.user {
		align-self: flex-end;
		align-items: flex-end;
	}
	.chat-line.assistant {
		align-self: flex-start;
		align-items: flex-start;
	}
	.chat-role {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.chat-role :global(svg) {
		width: 13px;
		height: 13px;
	}
	.chat-line.ok .chat-role {
		color: var(--success, var(--text-muted));
	}
	.chat-line.fail .chat-role {
		color: var(--danger, var(--text-muted));
	}
	.chat-bubble {
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
		line-height: 1.45;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.chat-line.user .chat-bubble {
		background: var(--accent-soft);
		color: var(--accent-text, var(--text));
	}
	.chat-line.assistant .chat-bubble {
		background: var(--bg-sunken);
		color: var(--text);
	}
	.chat-line.assistant.checking .chat-bubble {
		color: var(--text-muted);
	}
	.chat-line.assistant.fail .chat-bubble {
		background: var(--danger-soft, var(--bg-sunken));
		color: var(--danger, var(--text));
	}
	.check {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		min-width: 0;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		background: var(--surface-2);
		font-size: var(--text-sm);
		overflow-wrap: anywhere;
	}
	.check :global(svg) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}
	.check.ok {
		color: var(--success, var(--text));
	}
	.check.fail {
		color: var(--danger, var(--text));
	}
	.center {
		display: grid;
		place-items: center;
		padding: var(--space-8);
	}
	.crumb {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin: calc(var(--space-4) * -1) 0 var(--space-2) calc(var(--space-2) * -1);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	h1 {
		font-size: var(--text-xl);
	}
	h2 {
		margin: var(--space-6) 0 var(--space-2);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.btns {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.dirty {
		font-size: var(--text-sm);
		color: var(--accent-text);
	}
	.lead {
		margin: var(--space-1) 0 var(--space-4);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.card {
		padding: var(--space-4) var(--space-5);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	.inline {
		display: flex;
		gap: var(--space-2);
	}
	.inline > :global(:first-child) {
		flex: 1;
	}
	.sa {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.sa-icon {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		flex: none;
		border-radius: var(--radius-md);
		background: var(--surface);
		color: var(--text-muted);
	}
	.sa-txt {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
	}
	.sa-txt small {
		color: var(--text-muted);
		font-size: var(--text-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.warn {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-3);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	@media (max-width: 640px) {
		.two {
			grid-template-columns: 1fr;
		}
	}
</style>
