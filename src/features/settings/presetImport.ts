// 프리셋·정규식 JSON 가져오기 (설정 > 프리셋과 대화 패널의 프리셋 탭이 같이 씀)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { localizeImportWarning } from "$lib/utils/backendI18n";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";
import { library } from "$features/character/library.svelte";

const src = { source: "preset" };

/**
 * 여러 파일 가져오기: 종류를 먼저 보고 프리셋부터 가져온 뒤, 정규식 파일은 그 프리셋에 묶는다.
 * 프리셋 안에 든 정규식(SillyTavern·SPreset 등)은 백엔드가 프리셋과 함께 가져온다.
 * 프리셋이 없거나 여러 개면 정규식은 전역으로 간다.
 * 프리셋이 정확히 하나 들어왔으면 그 id를 돌려준다 (없거나 여러 개면 null).
 */
export async function importPresetPaths(paths: string[]): Promise<string | null> {
	const kinds = await Promise.all(paths.map((p) => call(commands.detectSillytavern(p), { ...src, silent: true }).catch(() => null)));
	const presets = paths.filter((_, i) => kinds[i] === "preset");
	const regexes = paths.filter((_, i) => kinds[i] === "regex");
	const others = paths.filter((_, i) => kinds[i] !== "preset" && kinds[i] !== "regex");
	let bindTo: string | null = null;
	let added = 0;
	for (const path of presets) {
		const r = await call(commands.importSillytavern(path, null, null, null), src).catch(() => null);
		if (r?.kind !== "preset") continue;
		added++;
		bindTo = presets.length === 1 ? r.presetId : null;
		const t = get(_);
		toasts.success(
			r.regexCount
				? t("settings.extra.presetImportedWithRegexToast", { values: { name: r.name, blocks: r.blockCount, regex: r.regexCount } })
				: t("settings.extra.presetImportedToast", { values: { name: r.name, blocks: r.blockCount } }),
		);
		for (const w of r.warnings) toasts.info(localizeImportWarning(w));
	}
	let regexCount = 0;
	for (const path of regexes) {
		const r = await call(commands.importSillytavern(path, null, null, bindTo), src).catch(() => null);
		if (r?.kind === "regex") regexCount += r.scripts.length;
	}
	if (regexCount) toasts.success(get(_)(bindTo ? "settings.extra.regexImportedBoundToast" : "settings.extra.regexImportedGlobalToast", { values: { count: regexCount } }));
	if (others.length) toasts.info(get(_)("settings.extra.presetImportSkippedToast"));
	library.presets = await call(commands.listPromptPresets(), src);
	return added === 1 ? bindTo : null;
}
