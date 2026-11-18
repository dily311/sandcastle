// StoryPlan(각본가, 게임 ADR-019) 화면 상태. 플랜 데이터는 백엔드가 저장하고, 여기는 마지막으로 읽은 것과 진행 중 표시만 가진다.
// 켜진 게임에서만 쓴다: 꺼진 게임은 아무것도 읽거나 부르지 않는다 (Lazy Initialization).
import type { StoryPlanRecord } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { gameStore } from "./gameStore.svelte";
import { runtimeStore } from "./runtimeStore.svelte";

const src = { source: "game" };

/** 게임 설정(settingsJson)의 storyPlan 칸 */
export function planSettings(settingsJson: string | Record<string, unknown> | null | undefined): { enabled: boolean; everyNTurns: number } {
	try {
		const raw = typeof settingsJson === "string" ? JSON.parse(settingsJson || "{}") : (settingsJson ?? {});
		const v = (raw as { storyPlan?: { enabled?: unknown; everyNTurns?: unknown } }).storyPlan;
		const n = typeof v?.everyNTurns === "number" ? Math.min(100, Math.max(3, Math.round(v.everyNTurns))) : 10;
		return { enabled: v?.enabled === true, everyNTurns: n };
	} catch {
		return { enabled: false, everyNTurns: 10 };
	}
}

class StoryPlanStore {
	record = $state<StoryPlanRecord | null>(null);
	busy = $state(false);
	/** 체크포인트마다 알아서 부르기. 기본 꺼짐 — 켜면 체크포인트마다 요금이 든다 (작업 기록에 자동 실행으로 남음) */
	autoRefresh = $state(false);

	get enabled() {
		return planSettings(gameStore.project?.settingsJson).enabled;
	}
	get due() {
		return this.enabled && runtimeStore.state?.storyPlanDue === true;
	}

	async load() {
		if (!this.enabled) return;
		this.record = await call(commands.getStoryPlan(), { source: "game", silent: true }).catch(() => this.record);
	}

	/** 새 플랜 세우기. 사용자가 누른 것(auto=false)과 자동 갱신(auto=true)을 작업 기록에서 구분한다 */
	async refresh(auto: boolean) {
		if (!this.enabled || this.busy) return;
		this.busy = true;
		try {
			const r = await call(commands.refreshStoryPlan(!auto, auto), src).catch(() => null);
			if (r) this.record = r;
			// 체크포인트 표시는 백엔드가 지운다 — 화면에서도 내린다
			const st = runtimeStore.state;
			if (st?.storyPlanDue) await runtimeStore.setState({ state: { ...st, storyPlanDue: false } });
		} finally {
			this.busy = false;
		}
	}

	reset() {
		this.record = null;
		this.busy = false;
	}
}

export const storyPlan = new StoryPlanStore();
