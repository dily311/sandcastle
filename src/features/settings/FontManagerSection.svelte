<script lang="ts">
	// 글꼴 관리 페이지: UI / 텍스트 / 모노스페이스 글꼴 지정 + 글꼴 파일 올리기·미리보기·삭제
	import { _ } from "svelte-i18n";
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
	const uiOptions = $derived([{ value: "", label: $_("settings.more2.fontUiDefault") }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);
	const textOptions = $derived([{ value: "", label: $_("settings.more2.fontTextDefault") }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);
	const monoOptions = $derived([{ value: "", label: $_("settings.more2.fontMonoDefault") }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);

	$effect(() => declarePreviewFonts(fonts));

	async function uploadFonts() {
		await addFonts(await pickFiles([{ name: $_("settings.more2.fontFilter"), extensions: ["ttf", "otf", "woff", "woff2"] }], true));
	}
	async function addFonts(paths: string[]) {
		if (!paths.length) return;
		const before = new Set(fonts.map((f) => f.path));
		await theme.addFonts(paths, false);
		const added = (theme.setting.fonts ?? []).filter((f) => !before.has(f.path));
		if (added.length === 1) toasts.success($_("settings.more2.fontUploadedOne", { values: { name: added[0].name } }));
		else if (added.length > 1) toasts.success($_("settings.more2.fontUploadedMany", { values: { count: added.length } }));
	}
	async function removeFont(path: string, name: string) {
		if (!(await confirmAction({ title: $_("settings.more2.deleteFontTitle"), message: $_("settings.more2.deleteFontMsg", { values: { name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await theme.removeFont(path);
	}

	$effect(() =>
		drops.register({
			label: $_("settings.more2.dropFontLabel"),
			hint: $_("settings.more2.dropFontHint"),
			accept: isFont,
			ondrop: async (files) => addFonts(await Promise.all(files.map((f) => storeFile(f)))),
		}),
	);

	function back() {
		nav.go({ page: "settings", tab: "theme" });
	}
</script>

<Section title={$_("settings.more2.fontMgrTitle")} lead={$_("settings.more2.fontMgrLead")}>
	{#snippet actions()}
		<Button size="sm" variant="ghost" onclick={back}>{#snippet icon()}<ArrowLeft />{/snippet}{$_("settings.more2.backToTheme")}</Button>
		<Button size="sm" variant="primary" onclick={uploadFonts}>{#snippet icon()}<Upload />{/snippet}{$_("settings.more2.uploadFontBtn")}</Button>
	{/snippet}

	<Group title={$_("settings.more2.fontAssignGroup")}>
		<Row label={$_("settings.more2.uiFont")} desc={$_("settings.more2.uiFontDesc")}>
			<Select value={theme.setting.appFont ?? ""} options={uiOptions} aria-label={$_("settings.more2.uiFont")} onchange={(e) => theme.update({ appFont: e.currentTarget.value || null })} />
		</Row>
		<Row label={$_("settings.more2.textFont")} desc={$_("settings.more2.textFontDesc")}>
			<Select value={theme.setting.textFont ?? ""} options={textOptions} aria-label={$_("settings.more2.textFont")} onchange={(e) => theme.update({ textFont: e.currentTarget.value || null })} />
		</Row>
		<Row label={$_("settings.more2.monoFont")} desc={$_("settings.more2.monoFontDesc")}>
			<Select value={theme.setting.monoFont ?? ""} options={monoOptions} aria-label={$_("settings.more2.monoFont")} onchange={(e) => theme.update({ monoFont: e.currentTarget.value || null })} />
		</Row>
		<div class="preview">
			<div class="pv-row">
				<span class="pv-label">UI</span>
				<span class="pv-ui">{$_("settings.more2.pvUiSample")}</span>
			</div>
			<div class="pv-row">
				<span class="pv-label">{$_("settings.more2.pvTextLabel")}</span>
				<span class="pv-text">{$_("settings.more2.pvTextSample")}</span>
			</div>
			<div class="pv-row">
				<span class="pv-label">{$_("settings.more2.pvMonoLabel")}</span>
				<code class="pv-mono">const font = "monospace"; // 0O 1lI 1234567890</code>
			</div>
		</div>
	</Group>

	<Group title={$_("settings.more2.uploadedFontsGroup", { values: { count: fonts.length } })}>
		{#snippet aside()}<Button size="sm" onclick={uploadFonts}>{#snippet icon()}<Upload />{/snippet}{$_("settings.more2.uploadFontBtn")}</Button>{/snippet}
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
								<span class="sample" style:font-family={`"${fontFamilyOf(f.path)}", var(--font-sans)`}>가나다라마바사 ABC abc 123<!-- i18n-ignore: 한글·영문·숫자 글꼴 모양 견본 --></span>
							</div>
							<div class="font-btns">
								<Button size="sm" variant={isUi ? "primary" : "secondary"} onclick={() => theme.update({ appFont: isUi ? null : f.path })}>UI</Button>
								<Button size="sm" variant={isText ? "primary" : "secondary"} onclick={() => theme.update({ textFont: isText ? null : f.path })}>{$_("settings.more2.pvTextLabel")}</Button>
								<Button size="sm" variant={isMono ? "primary" : "secondary"} onclick={() => theme.update({ monoFont: isMono ? null : f.path })}>{$_("settings.more2.pvMonoLabel")}</Button>
								<Button size="sm" variant="ghost" aria-label={$_("settings.more.deleteNamed", { values: { name: f.name } })} onclick={() => removeFont(f.path, f.name)}>{#snippet icon()}<Trash2 />{/snippet}</Button>
							</div>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="font-empty">{$_("settings.more2.noUploadedFonts")}</p>
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
