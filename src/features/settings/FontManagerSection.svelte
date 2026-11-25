<script lang="ts">
	// 글꼴 관리 페이지: UI / 텍스트 / 모노스페이스 글꼴 지정 + 글꼴 파일 올리기·미리보기·삭제
	import { ArrowLeft, Trash2, Upload } from "@lucide/svelte";
	import { pickFiles } from "$shared/ipc/dialog";
	import { drops, isFont, storeFile } from "$app/drop.svelte";
	import { declarePreviewFonts, fontFamilyOf } from "$shared/ui/userFont";
	import { Button, Select } from "$shared/ui";
	import { theme } from "$app/theme.svelte";
	import { nav } from "$app/nav.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const fonts = $derived(theme.setting.fonts ?? []);
	const uiOptions = $derived([{ value: "", label: "기본 (Pretendard)" }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);
	const textOptions = $derived([{ value: "", label: "UI 글꼴 따름" }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);
	const monoOptions = $derived([{ value: "", label: "기본 (JetBrains Mono)" }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);

	$effect(() => declarePreviewFonts(fonts));

	async function uploadFonts() {
		await addFonts(await pickFiles([{ name: "글꼴", extensions: ["ttf", "otf", "woff", "woff2"] }], true));
	}
	async function addFonts(paths: string[]) {
		if (!paths.length) return;
		const before = new Set(fonts.map((f) => f.path));
		await theme.addFonts(paths, false);
		const added = (theme.setting.fonts ?? []).filter((f) => !before.has(f.path));
		if (added.length === 1) toasts.success(`글꼴 '${added[0].name}'을(를) 올림`);
		else if (added.length > 1) toasts.success(`글꼴 ${added.length}개를 올림`);
	}
	async function removeFont(path: string, name: string) {
		if (!(await confirmAction({ title: "글꼴 삭제", message: `'${name}' 글꼴을 목록에서 지우고 복사해 둔 파일도 지움 (올릴 때 고른 원본 파일은 그대로)`, confirmLabel: "삭제", danger: true }))) return;
		await theme.removeFont(path);
	}

	$effect(() =>
		drops.register({
			label: "글꼴 올리기",
			hint: "TTF·OTF·WOFF·WOFF2 파일을 놓으면 글꼴 목록에 추가됨",
			accept: isFont,
			ondrop: async (files) => addFonts(await Promise.all(files.map((f) => storeFile(f)))),
		}),
	);

	function back() {
		nav.go({ page: "settings", tab: "theme" });
	}
</script>

<Section title="글꼴 관리" lead="UI·텍스트·모노스페이스 글꼴을 따로 고르고, 올린 글꼴 파일을 관리함">
	{#snippet actions()}
		<Button size="sm" variant="ghost" onclick={back}>{#snippet icon()}<ArrowLeft />{/snippet}테마로</Button>
		<Button size="sm" variant="primary" onclick={uploadFonts}>{#snippet icon()}<Upload />{/snippet}글꼴 올리기</Button>
	{/snippet}

	<Group title="글꼴 지정">
		<Row label="UI 글꼴" desc="메뉴·버튼·제목 등 앱 인터페이스 전반에 쓰이는 글꼴">
			<Select value={theme.setting.appFont ?? ""} options={uiOptions} aria-label="UI 글꼴" onchange={(e) => theme.update({ appFont: e.currentTarget.value || null })} />
		</Row>
		<Row label="텍스트 글꼴" desc="대화 본문·읽기 화면·캐릭터 소개에 쓰이는 글꼴">
			<Select value={theme.setting.textFont ?? ""} options={textOptions} aria-label="텍스트 글꼴" onchange={(e) => theme.update({ textFont: e.currentTarget.value || null })} />
		</Row>
		<Row label="모노스페이스 글꼴" desc="코드 블록·로그·정규식·고정폭 편집기에 쓰이는 글꼴">
			<Select value={theme.setting.monoFont ?? ""} options={monoOptions} aria-label="모노스페이스 글꼴" onchange={(e) => theme.update({ monoFont: e.currentTarget.value || null })} />
		</Row>
		<div class="preview">
			<div class="pv-row">
				<span class="pv-label">UI</span>
				<span class="pv-ui">버튼과 사이드바 메뉴 미리보기 · Sandcastle 0.3</span>
			</div>
			<div class="pv-row">
				<span class="pv-label">텍스트</span>
				<span class="pv-text">“바람이 숲 사이로 스며들며 조용한 밤이 찾아왔다.” The quick brown fox 123.</span>
			</div>
			<div class="pv-row">
				<span class="pv-label">모노</span>
				<code class="pv-mono">const font = "monospace"; // 0O 1lI 1234567890</code>
			</div>
		</div>
	</Group>

	<Group title="올린 글꼴 파일 ({fonts.length})">
		{#snippet aside()}<Button size="sm" onclick={uploadFonts}>{#snippet icon()}<Upload />{/snippet}글꼴 올리기</Button>{/snippet}
		<div class="list-wrap">
			{#if fonts.length}
				<ul class="fonts">
					{#each fonts as f (f.path)}
						{@const isUi = theme.setting.appFont === f.path}
						{@const isText = theme.setting.textFont === f.path}
						{@const isMono = theme.setting.monoFont === f.path}
						<li class:on={isUi || isText || isMono}>
							<div class="font-txt">
								<b>{f.name}</b>
								<span class="sample" style:font-family={`"${fontFamilyOf(f.path)}", var(--font-sans)`}>가나다라마바사 ABC abc 123</span>
							</div>
							<div class="font-btns">
								<Button size="sm" variant={isUi ? "primary" : "secondary"} onclick={() => theme.update({ appFont: isUi ? null : f.path })}>UI</Button>
								<Button size="sm" variant={isText ? "primary" : "secondary"} onclick={() => theme.update({ textFont: isText ? null : f.path })}>텍스트</Button>
								<Button size="sm" variant={isMono ? "primary" : "secondary"} onclick={() => theme.update({ monoFont: isMono ? null : f.path })}>모노</Button>
								<Button size="sm" variant="ghost" aria-label="{f.name} 삭제" onclick={() => removeFont(f.path, f.name)}>{#snippet icon()}<Trash2 />{/snippet}</Button>
							</div>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="font-empty">올린 글꼴이 없음. TTF·OTF·WOFF·WOFF2 파일을 올리거나 이 화면에 끌어다 놓으세요</p>
			{/if}
		</div>
	</Group>
</Section>

<style>
	.preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) 0;
		border-top: 1px solid var(--border);
	}
	.pv-row {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
	}
	.pv-label {
		width: 48px;
		flex: none;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.pv-ui {
		font-family: var(--font-sans);
		font-size: var(--text-sm);
	}
	.pv-text {
		font-family: var(--font-text);
		font-size: var(--text-md);
		line-height: 1.6;
	}
	.pv-mono {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.list-wrap {
		padding: var(--space-4) 0;
	}
	.fonts {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.fonts li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.fonts li.on {
		border-color: var(--accent);
	}
	.font-txt {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 2px;
		min-width: 180px;
	}
	.font-txt b {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sample {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-lg);
	}
	.font-btns {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex: none;
	}
	.font-empty {
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
</style>
