// 채팅 파일(.jsonl) 여러 개 가져오기 (5-E-4): 파일마다 파일 작업 대기열(5-E-28)에 하나씩 붙인다.
// 진행 막대·빼기·요약 알림은 대기열이 맡는다. 한 파일일 때만 그 대화를 연다
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { storeFile } from "$app/drop.svelte";
import { jobs, SKIPPED } from "$shared/stores/jobs.svelte";
import { library } from "./library.svelte";

const t = (key: string) => get(_)(key);

class ChatImport {
	/**
	 * 경로 또는 끌어다 놓은 파일. 하나면 가져온 뒤 `open`으로 그 대화를 연다.
	 * 여럿이면 대화를 옮겨 다니지 않고, 끝나면 대기열이 한 번 요약해 알린다
	 */
	async run(sources: (string | File)[], characterId: string | null, open?: (convId: string) => void) {
		if (!sources.length) return;
		const path = (s: string | File) => (typeof s === "string" ? s : storeFile(s));
		const name = (s: string | File) => (typeof s === "string" ? (s.split(/[\\/]/).pop() ?? s) : s.name);
		const many = sources.length > 1;
		const results = await Promise.allSettled(
			sources.map((s) =>
				jobs.add(
					name(s),
					async () => {
						const r = await library.importSillyTavern(await path(s), characterId, { quiet: many });
						if (r?.kind !== "chat") throw new Error(t("chatExtra.notAChatFile"));
						return r;
					},
					{ group: t("chatExtra.importChatsGroup") },
				),
			),
		);
		if (many) return void (await library.load());
		const r = results[0];
		if (r.status === "fulfilled" && r.value !== SKIPPED) open?.(r.value.conversation.id);
	}
}

export const chatImport = new ChatImport();
