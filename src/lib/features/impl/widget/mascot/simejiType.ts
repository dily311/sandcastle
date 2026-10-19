// 행동 타입 정의
export type BehaviorMode = "automatic" | "manual";
export type BehaviorType = "idle" | "walk" | "action" | "sitting" | "sleep" | "working";
export type CharacterAnimation = "idle" | "walk" | "sleep" | "sitting" | "working";

export interface Behavior {
	id: string;
	name: string;
	type: BehaviorType;

	/**
	 * 랜덤 선택 확률
	 */
	weight: number;

	/**
	 * 최소 지속 시간
	 */
	minDuration: number;

	/**
	 * 최대 지속 시간
	 */
	maxDuration: number;
}

export interface UserAction {
	id: string;
	name: string;
	animation: string;
	weight: number;
	cooldown: number;

	conditions?: {
		time?: string[];
		mood?: string[];
	};
}
