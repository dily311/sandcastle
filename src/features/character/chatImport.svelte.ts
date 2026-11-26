// 채팅 파일(.jsonl) 가져오기 (5-E-4, 1-J-6): 클릭 해제 가능한 진행률 토스트 + Android 포그라운드 알림 막대로 표시한다.
// 한 파일일 때만 그 대화를 열고, 여러 파일이면 끝난 뒤 한 번 요약해 알린다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { storeFile } from "$app/drop.svelte";
import { runWithStProgress } from "$app/syncProgress.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library } from "./library.svelte";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

class ChatImport {
	/**
	 * 경로 또는 끌어다 놓은 파일. 하나면 가져온 뒤 `open`으로 그 대화를 연다.
	 * 여럿이면 대화를 옮겨 다니지 않고, 끝나면 한 번 요약해 알린다
	 */
	async run(sources: (string | File)[], characterId: string | null, open?: (convId: string) => void) {
		if (!sources.length) return;
		const path = (s: string | File) => (typeof s === "string" ? s : storeFile(s));
		const many = sources.length > 1;
		let ok = 0;
		let failed = 0;
		let firstConvId: string | null = null;

		await runWithStProgress(
			{
				kind: "import",
				initialPhase: "chats",
				initialTotal: many ? sources.length : undefined,
				manualCounter: many,
				delayToastMs: many ? 0 : 150,
			},
			async (step) => {
				for (let i = 0; i < sources.length; i++) {
					try {
						const r = await library.importSillyTavern(await path(sources[i]), characterId, { quiet: many });
						if (r?.kind !== "chat") throw new Error(t("chatExtra.notAChatFile"));
						ok++;
						firstConvId ??= r.conversation.id;
					} catch {
						failed++;
					}
					if (many) step("chats", i + 1, sources.length);
				}
			},
		);

		if (many) {
			await library.load();
			const summary = `${t("jobs.doneItem", { name: t("chatExtra.importChatsGroup"), ok })}${failed ? t("jobs.failedPart", { count: failed }) : ""}`;
			toasts.show({ tone: failed ? "warning" : "success", message: summary, source: t("jobs.source") });
			return;
		}
		if (firstConvId) open?.(firstConvId);
	}
}

export const chatImport = new ChatImport();
