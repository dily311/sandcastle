// 게임 화면 효과·캐릭터 모션 (5-E-22·23). 백엔드가 대사 줄의 큐로 보낸 `effect`·`motion`을 그 줄이 보일 때 실행한다.
// 한 번 하고 끝나는 효과(떨림·번쩍·암전·페이드·폭죽)는 key를 올려 다시 그리게 하고,
// 계속되는 효과(꽃잎·눈·비·안개)는 `clear`나 다른 계속 효과가 올 때까지 남는다 (이 플레이 화면 동안만 — 다시 열면 없음)

export type Ambient = "petals" | "snow" | "rain" | "fog";
const AMBIENT: Ambient[] = ["petals", "snow", "rain", "fog"];
export const ONE_SHOT = ["shake", "flash", "blackout", "fade_in", "fireworks"] as const;
export type OneShot = (typeof ONE_SHOT)[number];
export const MOTIONS = ["jump", "bounce", "shake", "nod", "sway", "shrink", "slide_in", "slide_out"] as const;

/** "2s" / "500ms" / "1.5" → 밀리초 (모르면 기본값, 0.1초~10초로 묶음) */
export function durationMs(param: string | null | undefined, fallback: number): number {
	const m = /^\s*(\d+(?:\.\d+)?)\s*(ms|s)?\s*$/i.exec(param ?? "");
	if (!m) return fallback;
	const n = Number(m[1]) * (m[2]?.toLowerCase() === "ms" ? 1 : 1000);
	return Math.min(10_000, Math.max(100, n));
}

class StageFx {
	/** 한 번짜리 효과: 이름 → { key, ms } */
	shot = $state<Partial<Record<OneShot, { key: number; ms: number }>>>({});
	ambient = $state<Ambient | null>(null);
	/** 캐릭터 id → 지금 모션 */
	motions = $state<Record<string, { name: string; key: number }>>({});
	private seq = 0;

	effect(name: string, param?: string | null) {
		const n = name.trim().toLowerCase();
		if (n === "clear") return void (this.ambient = null);
		if ((AMBIENT as string[]).includes(n)) return void (this.ambient = n as Ambient);
		if ((ONE_SHOT as readonly string[]).includes(n)) {
			const def = n === "blackout" ? 1200 : n === "fade_in" ? 900 : n === "fireworks" ? 1800 : n === "flash" ? 350 : 500;
			this.shot = { ...this.shot, [n]: { key: ++this.seq, ms: durationMs(param, def) } };
		}
	}

	motion(characterId: string, name: string) {
		const n = name.trim().toLowerCase();
		if (!(MOTIONS as readonly string[]).includes(n)) return;
		this.motions = { ...this.motions, [characterId]: { name: n, key: ++this.seq } };
	}

	/** 이 줄의 효과·모션 큐를 실행 */
	run(cues: unknown[] | undefined) {
		for (const raw of cues ?? []) {
			const c = raw as { type?: string; name?: string; param?: string | null; characterId?: string };
			const t = (c.type ?? "").toLowerCase();
			if (t === "effect" && c.name) this.effect(c.name, c.param);
			else if (t === "motion" && c.name && c.characterId) this.motion(c.characterId, c.name);
		}
	}

	reset() {
		this.shot = {};
		this.ambient = null;
		this.motions = {};
	}
}

export const stageFx = new StageFx();
