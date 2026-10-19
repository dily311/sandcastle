import { tick } from "svelte";
import type { Attachment } from "svelte/attachments";
import { innerWidth, innerHeight } from "svelte/reactivity/window";

type Pos = { x: number; y: number };

type SnapAxis = {
	/** 화면 밖으로 삐져나갈 크기(px). 이만큼만 남기고 숨김 */
	offset?: number;
};

type SnapState = { x: "left" | "right" | null; y: "top" | "bottom" | null };
type SnapOptions = {
	x?: boolean | SnapAxis;
	y?: boolean | SnapAxis;
	/** 스냅 상태가 바뀔 때 호출. side가 null이면 스냅 해제 */
	onchange?: (state: SnapState) => void;
};

type Options = {
	target?: HTMLElement | null;
	startX?: number;
	startY?: number;
	pos?: Pos;
	/** 생략하면 스냅 없음 */
	snap?: SnapOptions;
	onDragEnd?: (pos: { x: number; y: number }) => void;
};

function normalizeAxis(v: boolean | SnapAxis | undefined) {
	if (!v) return null;
	return { offset: v === true ? 0 : (v.offset ?? 0) };
}

export function draggable(options: Options = {}): Attachment<HTMLElement> {
	return (node) => {
		const moveTarget = options.target ?? node;

		const snapX = normalizeAxis(options.snap?.x);
		const snapY = normalizeAxis(options.snap?.y);

		// $state는 변수 선언의 초기값으로만 쓸 수 있어서 fallback을 따로 만듭니다
		const fallback = $state({ x: options.startX ?? 10, y: options.startY ?? 10 });
		const pos = options.pos ?? fallback;

		let lastX = 0;
		let lastY = 0;
		let current = { x: null, y: null } as SnapState;
		let stopDrag: (() => void) | null = null;
		let hasMoved = false;

		// pos → DOM 자동 반영 (외부에서 pos를 바꿔도 요소가 따라 움직임)
		$effect(() => {
			moveTarget.style.left = `${pos.x}px`;
			moveTarget.style.top = `${pos.y}px`;

			// 가장자리 감지 (1px 오차 허용)
			const w = moveTarget.offsetWidth;
			const h = moveTarget.offsetHeight;
			const iw = innerWidth.current ?? 0;
			const ih = innerHeight.current ?? 0;

			moveTarget.classList.toggle("snap-left", pos.x <= 1);
			moveTarget.classList.toggle("snap-right", pos.x + w >= iw - 1);
			moveTarget.classList.toggle("snap-top", pos.y <= 1);
			moveTarget.classList.toggle("snap-bottom", pos.y + h >= ih - 1);
		});

		function emit(next: SnapState) {
			if (next.x === current.x && next.y === current.y) return;
			current = next;
			options.snap?.onchange?.(next);
		}

		function handleSnapOn() {
			const rect = moveTarget.getBoundingClientRect();
			let next = { x: null, y: null } as SnapState;
			if (rect.right < window.innerWidth && snapX) {
				const sw = rect.width - snapX.offset < 0 ? 0 : snapX.offset;
				const x = window.innerWidth - (rect.width - sw);
				pos.x = x;
				next.x = "right";
				options.onDragEnd?.({ x, y: pos.y });

				moveTarget.removeEventListener("dblclick", handleSnapOn);
				moveTarget.addEventListener("dblclick", handleSnapCancel);
			}
			emit(next);
		}

		function handleSnapCancel() {
			const rect = moveTarget.getBoundingClientRect();
			if (rect.right > window.innerWidth) {
				const x = window.innerWidth - rect.width - 10;
				pos.x = x;
				options.onDragEnd?.({ x, y: pos.y });
				moveTarget.addEventListener("dblclick", handleSnapOn);
				moveTarget.removeEventListener("dblclick", handleSnapCancel);
			}
			emit({ x: null, y: null });
		}

		function handleMouseDown(e: MouseEvent) {
			if (e.button !== 0) return;

			const target = e.target as Element;
			if (node !== target && target.closest("button, a, input, .no-drag")) return;

			moveTarget.style.transition = "none";
			moveTarget.classList.add("drag");

			const offsetX = e.clientX - pos.x;
			const offsetY = e.clientY - pos.y;

			function handleMouseMove(e: MouseEvent) {
				const nextX = e.clientX - offsetX;
				const nextY = e.clientY - offsetY;

				// 마우스가 실제로 움직였을 때만 판정 (미세한 떨림 방지를 위해 1~2px 이상 움직였을 때로 설정 가능)
				if (nextX !== pos.x || nextY !== pos.y) {
					hasMoved = true;
				}

				pos.x = nextX;
				pos.y = nextY;
			}

			function cleanup() {
				moveTarget.removeEventListener("click", handleSnapOn);
				moveTarget.classList.remove("drag");
				window.removeEventListener("mousemove", handleMouseMove);
				window.removeEventListener("mouseup", handleMouseUp);
				window.removeEventListener("scroll", cleanup);
				stopDrag = null;
			}

			async function handleMouseUp() {
				cleanup(); // 리스너와 드래그 미리 정리

				const { innerWidth, innerHeight } = window;
				moveTarget.style.transition =
					"left 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275), top 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275), border-radius 0.3s";
				let rect = moveTarget.getBoundingClientRect();
				let x = pos.x;
				let y = pos.y;

				if (options.snap) {
					let next = { x: null, y: null } as SnapState;
					if (snapX && lastX !== x && rect.right > innerWidth) next.x = "right";
					if (snapY && lastY !== y && rect.bottom > innerHeight) next.y = "bottom";
					emit(next);
					await tick();

					// 3) 바뀐 크기로 다시 측정해서 위치 계산
					rect = moveTarget.getBoundingClientRect();
					if (snapX && next.x === "right") {
						const sw = rect.width - snapX.offset < 0 ? 0 : snapX.offset;
						x = innerWidth - (rect.width - sw);
					}
					if (snapY && next.y === "bottom") {
						const sh = rect.height - snapY.offset < 0 ? 0 : snapY.offset;
						y = innerHeight - (rect.height - sh);
					}
					moveTarget.addEventListener("dblclick", handleSnapCancel);
				}

				// 탈출 방지
				if (x < 0) x = 0;
				else if (x > innerWidth) x = innerWidth - rect.width;
				if (y < 0) y = 0;
				else if (y > innerHeight) y = innerHeight - rect.height;

				lastX = x;
				lastY = y;

				// 최종 위치만 대입하면 위의 $effect가 스타일에 반영
				pos.x = x;
				pos.y = y;
				if (hasMoved) {
					options.onDragEnd?.({ x, y });
				}
				hasMoved = false;
			}

			stopDrag = cleanup;
			window.addEventListener("scroll", cleanup, { passive: true });
			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);
		}

		node.addEventListener("mousedown", handleMouseDown);

		// 정리 함수 (요소가 사라지거나 attachment가 다시 실행될 때 호출)
		return () => {
			stopDrag?.();
			node.removeEventListener("mousedown", handleMouseDown);
			moveTarget.removeEventListener("dblclick", handleSnapCancel);
		};
	};
}
