// 채팅 파일(.jsonl) 여러 개 가져오기 (5-E-4): 하나씩 차례로, 진행 막대·멈추기. 한 파일일 때만 그 대화를 연다
import { storeFile } from "$app/drop.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library } from "./library.svelte";

class ChatImport {
	total = $state(0);
	done = $state(0);
	failed = $state(0);
	current = $state("");
	stopping = $state(false);

	get running() {
		return this.total > 0;
	}

	/** 지금 파일이 끝나면 멈춤 (파일 하나는 한 번에 저장되어 중간에 끊지 않는다) */
	stop() {
		this.stopping = true;
	}

	/**
	 * 경로 또는 끌어다 놓은 파일. 하나면 지금처럼 가져온 뒤 `open`으로 그 대화를 연다.
	 * 여럿이면 대화를 옮겨 다니지 않고 끝에 한 번 알린다
	 */
	async run(sources: (string | File)[], characterId: string | null, open?: (convId: string) => void) {
		if (!sources.length || this.running) return;
		const path = (s: string | File) => (typeof s === "string" ? s : storeFile(s));
		if (sources.length === 1) {
			const r = await library.importSillyTavern(await path(sources[0]), characterId).catch(() => null);
			if (r?.kind === "chat") open?.(r.conversation.id);
			return;
		}
		this.total = sources.length;
		this.done = this.failed = 0;
		this.stopping = false;
		let messages = 0;
		try {
			for (const s of sources) {
				if (this.stopping) break;
				this.current = typeof s === "string" ? (s.split(/[\/]/).pop() ?? s) : s.name;
				const r = await Promise.resolve(path(s))
					.then((p) => library.importSillyTavern(p, characterId, { quiet: true }))
					.catch(() => null);
				if (r?.kind === "chat") messages += r.messageCount;
				else this.failed++;
				this.done++;
			}
			await library.load();
			const ok = this.done - this.failed;
			const left = this.total - this.done;
			toasts.show({
				tone: this.failed ? "warning" : "success",
				message: `대화 ${ok}개 가져옴 · 메시지 ${messages}개${this.failed ? ` · 실패 ${this.failed}개 (설정 > 로그)` : ""}${left ? ` · 멈춰서 ${left}개 남김` : ""}`,
			});
		} finally {
			this.total = 0;
			this.current = "";
		}
	}
}

export const chatImport = new ChatImport();
