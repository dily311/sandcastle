import { Character } from "../character/Character";
import type { Behavior } from "../simejiType";

export class CharacterManager {
	private characters: Map<string, Character> = new Map();
	private lastSentData: string = "";

	// 캐릭터 생성 및 등록
	async spawnCharacter(id: string, x: number, y: number, assetPath: string, behaviors: Behavior[]) {
		const char = new Character();
		await char.init(assetPath, behaviors);
		char.setPosition(x, y);
		
		this.characters.set(id, char);

		return char;
	}

	// 모든 캐릭터 업데이트 및 일괄 IPC 처리
	async update(deltaSeconds: number) {
		for (const [id, character] of this.characters) {
			character.update(deltaSeconds);
		}
	}

	removeCharacter(id: string) {
		const character = this.characters.get(id);
		if (character) {
			character.destroy(); // 픽시 오브젝트 정리
			this.characters.delete(id);
		}
	}
}
