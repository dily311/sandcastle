// 파일 작업 대기열 (5-E-28, ADR-027): 사용자가 누른 파일 작업(가져오기·지우기·백업·동기화…)을 앱 전체 한 줄로 하나씩.
// - DB·디스크 쓰기가 겹치지 않게 차례로 돈다. 앞 것이 도는 동안 누른 것은 줄 뒤에 붙는다 (거절하지 않음)
// - 실패해도 다음 것을 이어 하고, 줄이 빌 때 두 개 이상 했으면 요약 알림 한 번
// - 메모리에만 둔다: 앱을 끄면 기다리던 것은 버린다 (닫기 버튼이 남은 수를 알림)
// - AI 작업은 여기가 아니라 백엔드 TaskManager (요금·실행 주체 기록)
// 화면 상태(목록 펼침 등)는 JobProgress.svelte가 따로 가진다.
import { toasts } from "./toast.svelte";

export interface JobContext {
	/** 사용자가 지금 작업을 멈추라고 했나 (멈출 수 있는 작업만 물어본다) */
	stopped(): boolean;
}

export interface JobOptions {
	/** 요약 알림에서 묶는 이름 (예: "캐릭터 지우기"). 없으면 label */
	group?: string;
	/** 하던 중에 멈출 수 있나 (run이 ctx.stopped()를 보고 그만둠) */
	stoppable?: boolean;
}

export interface JobView {
	id: number;
	label: string;
	group: string;
	stoppable: boolean;
}

interface Job extends JobView {
	run: (ctx: JobContext) => Promise<unknown>;
	resolve: (v: unknown) => void;
	reject: (e: unknown) => void;
}

/** 대기열에서 빼서 하지 않은 작업 (add의 Promise가 이 값으로 끝남) */
export const SKIPPED = Symbol("skipped");

/** 이만큼 걸리면 하나뿐이어도 진행 막대를 보인다 (짧은 지우기 하나에 막대가 번쩍이지 않게) */
export const SHOW_AFTER_MS = 500;

class Jobs {
	/** 기다리는 작업 (지금 도는 것은 빼고) */
	waiting = $state<JobView[]>([]);
	current = $state<JobView | null>(null);
	stopping = $state(false);
	/** 줄이 빌 때까지 한 묶음: 한 것·실패·멈춤으로 건너뜀 */
	done = $state(0);
	failed = $state(0);
	/** 지금 작업이 SHOW_AFTER_MS를 넘겼나 */
	slow = $state(false);

	#queue: Job[] = [];
	#seq = 0;
	#running = false;
	/** 묶음별 결과 (요약 알림용) */
	#groups = new Map<string, { ok: number; failed: number; skipped: number }>();

	/** 지금 것 + 기다리는 것 */
	get pending(): number {
		return this.waiting.length + (this.current ? 1 : 0);
	}
	get total(): number {
		return this.done + this.pending;
	}
	/** 진행 막대를 보일 때: 둘 이상이거나 오래 걸림 */
	get visible(): boolean {
		return !!this.current && (this.total > 1 || this.slow);
	}

	/**
	 * 줄 뒤에 붙인다. 끝나면 run의 결과, 실패하면 그 오류로 끝나고, 대기 중에 빼면 SKIPPED.
	 * 오류 알림은 run 안의 call이 이미 띄우므로 여기서는 다시 띄우지 않는다
	 */
	add<T>(label: string, run: (ctx: JobContext) => Promise<T>, opts: JobOptions = {}): Promise<T | typeof SKIPPED> {
		return new Promise<T | typeof SKIPPED>((resolve, reject) => {
			const job: Job = {
				id: ++this.#seq,
				label,
				group: opts.group ?? label,
				stoppable: !!opts.stoppable,
				run,
				resolve: resolve as (v: unknown) => void,
				reject,
			};
			this.#queue.push(job);
			this.#sync();
			void this.#pump();
		});
	}

	/**
	 * add와 같지만 대기 중에 빼면 undefined (결과만 쓰는 화면용).
	 * 주의: 작업 안에서 다시 run/add를 기다리면 안 된다 — 한 줄이라 서로 기다리며 멈춘다.
	 * 그래서 화면이 누른 자리(또는 그 자리 하나에서만 부르는 store 메서드)에서만 감싼다
	 */
	async run<T>(label: string, run: (ctx: JobContext) => Promise<T>, opts: JobOptions = {}): Promise<T | undefined> {
		const r = await this.add(label, run, opts);
		return r === SKIPPED ? undefined : r;
	}

	/** 기다리는 작업 빼기 (지금 도는 것은 stop으로) */
	remove(id: number) {
		const i = this.#queue.findIndex((j) => j.id === id);
		if (i < 0) return;
		const [job] = this.#queue.splice(i, 1);
		this.#count(job.group, "skipped");
		job.resolve(SKIPPED);
		this.#sync();
	}

	/** 기다리는 것을 모두 빼기 */
	clear() {
		for (const j of [...this.#queue]) this.remove(j.id);
	}

	/** 지금 작업 멈추기 (멈출 수 있는 것만) */
	stop() {
		if (this.current?.stoppable) this.stopping = true;
	}

	#sync() {
		this.waiting = this.#queue.map(({ id, label, group, stoppable }) => ({ id, label, group, stoppable }));
	}

	#count(group: string, k: "ok" | "failed" | "skipped") {
		const g = this.#groups.get(group) ?? { ok: 0, failed: 0, skipped: 0 };
		g[k]++;
		this.#groups.set(group, g);
	}

	async #pump() {
		if (this.#running) return;
		this.#running = true;
		try {
			for (let job = this.#queue.shift(); job; job = this.#queue.shift()) {
				this.#sync();
				this.current = { id: job.id, label: job.label, group: job.group, stoppable: job.stoppable };
				this.stopping = false;
				this.slow = false;
				const timer = setTimeout(() => (this.slow = true), SHOW_AFTER_MS);
				try {
					job.resolve(await job.run({ stopped: () => this.stopping }));
					this.#count(job.group, "ok");
				} catch (e) {
					this.failed++;
					this.#count(job.group, "failed");
					job.reject(e);
				} finally {
					clearTimeout(timer);
					this.done++;
				}
			}
		} finally {
			this.current = null;
			this.#running = false;
			this.#summary();
		}
	}

	/** 줄이 비었을 때 한 번, 둘 이상 했을 때만 (하나뿐이면 그 작업의 알림·오류 알림으로 충분) */
	#summary() {
		const groups = [...this.#groups.entries()];
		const ran = groups.reduce((n, [, g]) => n + g.ok + g.failed + g.skipped, 0);
		const bad = groups.some(([, g]) => g.failed || g.skipped);
		if (ran > 1) {
			const parts = groups.map(([name, g]) => `${name} ${g.ok}개${g.failed ? ` · 실패 ${g.failed}개` : ""}${g.skipped ? ` · 뺌 ${g.skipped}개` : ""}`);
			toasts.show({ tone: bad ? "warning" : "success", message: parts.join(" / "), source: "작업 대기열" });
		}
		this.#groups.clear();
		this.done = this.failed = 0;
	}
}

export const jobs = new Jobs();
