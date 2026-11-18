// 대필 (5-D, ADR-018): 입력칸에 채우는 AI 초안의 화면 상태. 대화마다 따로 (대화를 바꿔도 다른 대화의 입력을 건드리지 않게)
import { Channel } from "@tauri-apps/api/core";
import type { StreamPayload } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";

const src = { source: "ghost" };

class GhostStore {
	/** 대필 중인 대화 */
	running = $state<Record<string, boolean>>({});
	/** 요청 전 입력 (원래대로) */
	before = $state<Record<string, string | null>>({});
	/** 마지막으로 채운 글 — 입력이 이것과 같을 때만 "원래대로"를 보인다 */
	result = $state<Record<string, string>>({});
	/** 대필을 거친 글이 입력칸에 있음 (보낼 때 meta.ghost) */
	used = $state<Record<string, boolean>>({});

	isRunning(convId: string) {
		return !!this.running[convId];
	}

	/** 초안(없을 수 있음)으로 AI에게 요청. 쓰는 대로 입력칸에 채우고, 실패하면 원래 입력으로 되돌린다. 마지막 글을 돌려준다(실패·멈춤은 null) */
	async run(convId: string, brief: string | null, getDraft: () => string, setDraft: (v: string) => void): Promise<string | null> {
		if (this.running[convId]) return null;
		const before = getDraft();
		this.before[convId] = before;
		this.running[convId] = true;
		let acc = "";
		const ch = new Channel<StreamPayload>();
		ch.onmessage = (p) => {
			acc += p.chunk;
			setDraft(acc);
		};
		try {
			const text = await call(commands.ghostwrite(convId, brief, ch), src);
			setDraft(text);
			this.result[convId] = text;
			this.used[convId] = true;
			return text;
		} catch {
			// 실패(토스트는 call이 띄움)·멈춤: 받은 글이 있으면 두고, 없으면 원래 입력으로
			if (acc.trim()) {
				this.result[convId] = acc;
				this.used[convId] = true;
			} else {
				setDraft(before);
				this.before[convId] = null;
			}
			return null;
		} finally {
			this.running[convId] = false;
		}
	}

	cancel(convId: string) {
		void call(commands.ghostwriteCancel(convId), { ...src, silent: true }).catch(() => {});
	}

	/** 원래 입력으로 */
	undo(convId: string, setDraft: (v: string) => void) {
		setDraft(this.before[convId] ?? "");
		this.before[convId] = null;
		this.used[convId] = false;
	}

	canUndo(convId: string, draft: string) {
		return !this.running[convId] && this.before[convId] != null && draft === this.result[convId];
	}

	/** 보낼 때: 대필을 거친 글이었나 (한 번 읽으면 지움) */
	takeUsed(convId: string) {
		const u = !!this.used[convId];
		this.used[convId] = false;
		this.before[convId] = null;
		return u;
	}

	/** 입력을 비웠으면 대필 표시를 거둔다 */
	cleared(convId: string) {
		this.used[convId] = false;
		this.before[convId] = null;
	}
}

export const ghost = new GhostStore();
