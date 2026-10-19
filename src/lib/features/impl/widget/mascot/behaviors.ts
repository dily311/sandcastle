import type { Behavior } from "./simejiType";

// 이후 이 배열을 DB에서 불러올 수 있도록 수정
export const behaviors: Behavior[] = [
	{
		id: "idle",
		name: "가만히 있기",

		type: "idle",

		weight: 50,

		minDuration: 1000,
		maxDuration: 3000,
	},

	{
		id: "walk",
		name: "걸어다니기",

		type: "walk",

		weight: 40,

		minDuration: 1000,
		maxDuration: 3000,
	},

	{
		id: "dance",
		name: "춤추기",

		type: "action",

		weight: 10,

		minDuration: 2000,
		maxDuration: 4000,
	},

	{
		id: "sleep",
		name: "잠자기",

		type: "action",

		weight: 20,

		minDuration: 3000,
		maxDuration: 6000,
	},
];
