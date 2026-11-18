// 고치기 제안 (6-D-3). 데이터(제안 목록)와 화면 상태(대화상자 열림)를 나눈다.
// 코어 플러그인 `community`가 켜져 있을 때만 구독한다(PluginHost가 watch/unwatch).
import type { ChangeProposal } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { onEvent } from "$shared/ipc/events";
import { toasts } from "$shared/stores/toast.svelte";

class Proposals {
	pending = $state<ChangeProposal[]>([]);
	/** 최근에 고른 것 (되돌리기용) */
	decided = $state<ChangeProposal[]>([]);
	private seen = new Set<string>();
	private unlisten: (() => void) | null = null;
	private watching = false;

	async watch() {
		if (this.watching) return;
		this.watching = true;
		const u = await onEvent("change-proposals-changed", () => void this.load(true));
		if (!this.watching) return u();
		this.unlisten = u;
		await this.load(false);
	}

	unwatch() {
		this.watching = false;
		this.unlisten?.();
		this.unlisten = null;
	}

	/** notify: 새로 생긴 제안이 있으면 알림 */
	async load(notify: boolean) {
		const all = await call(commands.listChangeProposals(null), { source: "proposals", silent: true }).catch(() => null);
		if (!all) return;
		this.pending = all.filter((p) => p.status === "pending");
		this.decided = all.filter((p) => p.status !== "pending").slice(0, 20);
		const fresh = this.pending.filter((p) => !this.seen.has(p.id));
		for (const p of this.pending) this.seen.add(p.id);
		if (notify && fresh.length > 0) {
			toasts.show({
				tone: "info",
				message: `플러그인이 고치기를 제안했습니다 (${this.pending.length}개 기다림)`,
				action: { label: "보기", run: () => (proposalsUi.open = true) },
			});
		}
	}

	async apply(id: string) {
		await call(commands.applyChangeProposal(id), { source: "proposals" }).catch(() => null);
		await this.load(false);
	}
	async reject(id: string) {
		await call(commands.rejectChangeProposal(id), { source: "proposals" }).catch(() => null);
		await this.load(false);
	}
	async revert(id: string) {
		await call(commands.revertChangeProposal(id), { source: "proposals" }).catch(() => null);
		await this.load(false);
	}
}

export const proposals = new Proposals();

/** 화면 상태: 대화상자 열림 */
export const proposalsUi = $state({ open: false });
