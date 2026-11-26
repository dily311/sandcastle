<script lang="ts">
	import { onMount } from "svelte";
	import { RefreshCw } from "@lucide/svelte";
	import { _, locale } from "svelte-i18n";
	import { AVAILABLE_LOCALES, setAppLocale, type AppLocale } from "$lib/utils/i18n";
	import type { UpdateInfo } from "$ipc/bindings";
	import { Button, Select, Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { checkForUpdate, openRelease } from "$app/updateCheck";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => appSetting.load());
	const s = $derived(appSetting.value);
	const LANGUAGE_LABELS: Record<string, string> = { ko: "한국어", en: "English" }; // i18n-ignore: 언어 고유 자칭(autonym)
	function localeLabel(code: string): string {
		if (LANGUAGE_LABELS[code]) return LANGUAGE_LABELS[code];
		try {
			return new Intl.DisplayNames([code], { type: "language" }).of(code) ?? code;
		} catch {
			return code;
		}
	}
	const languageOptions = $derived(
		[...AVAILABLE_LOCALES]
			.sort((a, b) => (a === "ko" ? -1 : b === "ko" ? 1 : a === "en" ? -1 : b === "en" ? 1 : a.localeCompare(b)))
			.map((code) => ({ value: code, label: localeLabel(code) })),
	);
	const currentLocale = $derived.by<AppLocale>(() => {
		const raw = ($locale ?? "en").toLowerCase();
		if (AVAILABLE_LOCALES.includes(raw)) return raw;
		const base = raw.split("-")[0];
		return AVAILABLE_LOCALES.includes(base) ? base : "en";
	});

	// 새 버전 확인 (1-E-2): 누를 때만 확인하고, 내려받기는 릴리스 페이지에서 직접
	let update = $state<UpdateInfo | null>(null);
	let checking = $state(false);
	async function check() {
		checking = true;
		update = await checkForUpdate().finally(() => (checking = false));
	}
	const status = $derived.by(() => {
		if (!update) return null;
		if (update.newer) return $_("settings.more.updateNewer", { values: { latest: update.latest ?? "", current: update.current } });
		if (!update.latest) return $_("settings.more.updateNoRelease", { values: { current: update.current } });
		return $_("settings.more.updateLatest", { values: { current: update.current } });
	});
</script>

<Section title={$_("settings.general.title")} lead={$_("settings.general.lead")}>
	{#if s}
		<Group>
			<Row label={$_("settings.general.language")} desc={$_("settings.general.languageDesc")}>
				<Select
					value={currentLocale}
					options={languageOptions}
					aria-label={$_("settings.general.language")}
					onchange={(e) => void setAppLocale(e.currentTarget.value as AppLocale)}
				/>
			</Row>
		</Group>
		<Group title={$_("settings.general.updates")}>
			<Row label={$_("settings.general.checkUpdate")} desc={status ?? $_("settings.general.checkUpdateDesc")}>
				{#if update?.newer}<Button size="sm" variant="primary" onclick={() => update && openRelease(update)}>{$_("settings.general.openRelease")}</Button>{/if}
				<Button size="sm" loading={checking} disabled={checking} onclick={check}>{#snippet icon()}<RefreshCw />{/snippet}{$_("settings.general.checkUpdateBtn")}</Button>
			</Row>
			<Row label={$_("settings.general.checkOnStart")} desc={$_("settings.general.checkOnStartDesc")} nowrap>
				<Switch checked={s.checkUpdatesOnStart} label={$_("settings.general.checkOnStart")} hideLabel onchange={(v) => appSetting.update({ checkUpdatesOnStart: v })} />
			</Row>
		</Group>
	{/if}
</Section>
