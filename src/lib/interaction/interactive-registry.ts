import { windowCommands } from "$/ipc/commands";

export interface Hitbox {
	x: number;
	y: number;
	width: number;
	height: number;
}

const EPSILON = 1;

class InteractiveManager {
	private elements = new Map<string, HTMLElement>();
	private hitboxes = new Map<string, Hitbox>();
	private dirty = new Set<string>();
	private removed = new Set<string>();

	private flushQueued = false;
	private sending = false;
	private animLoop = false;
	private ro?: ResizeObserver;

	constructor() {
		if (typeof window === "undefined") return;

		const measure = () => this.measureAll();

		// 전역 리스너는 여기서 딱 한 번만 등록
		window.addEventListener("resize", measure, true);
		window.addEventListener("scroll", measure, true);
		window.addEventListener("transitionrun", this.trackAnimations, true);
		window.addEventListener("animationstart", this.trackAnimations, true);

		// 크기 변화 (요소 자신 + 문서 전체 레이아웃)
		this.ro = new ResizeObserver(measure);
		this.ro.observe(document.documentElement);

		// DOM/스타일 변경 → microtask로 호출되므로 페인트 전에 즉시 반영됨
		new MutationObserver(measure).observe(document.documentElement, {
			childList: true,
			subtree: true,
			attributes: true,
		});

		// 폰트 로드 후 레이아웃 이동
		document.fonts?.addEventListener("loadingdone", measure);
	}

	// ---------------------------------------------------------
	// 등록 / 해제
	// ---------------------------------------------------------
	registerInteractiveElement(id: string, element: HTMLElement) {
		const prev = this.elements.get(id);
		if (prev && prev !== element) this.ro?.unobserve(prev);

		this.elements.set(id, element);
		this.ro?.observe(element);
		this.measure(id, element); // 등록 즉시 측정
	}

	unregisterInteractiveElement(id: string) {
		const el = this.elements.get(id);
		if (el) this.ro?.unobserve(el);
		this.elements.delete(id);
		this.drop(id);
	}

	// ---------------------------------------------------------
	// 측정
	// ---------------------------------------------------------
	measureAll = () => {
		for (const [id, el] of this.elements) this.measure(id, el);
	};

	private measure(id: string, el: HTMLElement) {
		const rect = el.getBoundingClientRect();

		// 숨겨졌거나 DOM에서 빠진 요소는 hitbox 제거
		if (!el.isConnected || (rect.width === 0 && rect.height === 0)) {
			this.drop(id);
			return;
		}

		const next: Hitbox = {
			x: rect.left,
			y: rect.top,
			width: rect.width + 10,
			height: rect.height + 10,
		};
		const prev = this.hitboxes.get(id);
		if (prev && !this.hasMeaningfulChange(prev, next)) return;

		this.hitboxes.set(id, next);
		this.dirty.add(id);
		this.removed.delete(id);
		this.queueFlush();
	}

	private drop(id: string) {
		if (!this.hitboxes.delete(id)) return;
		this.dirty.delete(id);
		this.removed.add(id);
		this.queueFlush();
	}

	private hasMeaningfulChange(a: Hitbox, b: Hitbox) {
		return (
			Math.abs(a.x - b.x) >= EPSILON ||
			Math.abs(a.y - b.y) >= EPSILON ||
			Math.abs(a.width - b.width) >= EPSILON ||
			Math.abs(a.height - b.height) >= EPSILON
		);
	}

	// transition / animation 동안은 매 프레임 재측정
	private trackAnimations = () => {
		if (this.animLoop) return;
		this.animLoop = true;
		const tick = () => {
			this.measureAll();
			if (document.getAnimations().length > 0) {
				requestAnimationFrame(tick);
			} else {
				this.animLoop = false;
			}
		};
		requestAnimationFrame(tick);
	};

	// ---------------------------------------------------------
	// 수동 hitbox (Pixi 등 DOM이 아닌 것)
	// ---------------------------------------------------------
	registerInteractiveHitbox(id: string, rect: Hitbox) {
		const prev = this.hitboxes.get(id);
		if (prev && !this.hasMeaningfulChange(prev, rect)) return;

		this.hitboxes.set(id, rect);
		this.dirty.add(id);
		this.removed.delete(id);
		this.queueFlush(); // microtask로 즉시 전송
	}

	unregisterInteractiveHitbox(id: string) {
		this.drop(id);
	}
	// ---------------------------------------------------------
	// 전송 (rAF/스로틀 없이 즉시, 전송 중이면 모아서 다음에)
	// ---------------------------------------------------------
	private queueFlush() {
		if (this.flushQueued) return;
		this.flushQueued = true;
		queueMicrotask(() => {
			this.flushQueued = false;
			void this.flush();
		});
	}

	private async flush() {
		if (this.sending) return; // 끝나면 finally에서 다시 확인
		if (this.dirty.size === 0 && this.removed.size === 0) return;

		this.sending = true;
		const upserts = Array.from(this.dirty, (id) => ({
			id,
			...this.hitboxes.get(id)!,
		}));
		const removedIds = Array.from(this.removed);
		this.dirty.clear();
		this.removed.clear();

		let failed = false;
		try {
			await windowCommands.updateInteractiveRegions({ upserts, removed: removedIds });
		} catch (e) {
			failed = true;
			console.error("updateInteractiveRegions failed", e);
			// 유실 방지: 실패분 복구
			for (const u of upserts) if (this.hitboxes.has(u.id)) this.dirty.add(u.id);
			for (const id of removedIds) if (!this.hitboxes.has(id)) this.removed.add(id);
		} finally {
			this.sending = false;
			if (failed) setTimeout(() => this.queueFlush(), 100);
			else if (this.dirty.size || this.removed.size) this.queueFlush();
		}
	}
}

export const interactiveManager = new InteractiveManager();