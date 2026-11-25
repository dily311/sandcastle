import type { MascotCharacterConfig_Serialize } from "$ipc/bindings";
import { Character, type CharacterCallbacks } from "../character/Character";

export class CharacterManager {
	private characters: Map<string, Character> = new Map();

	async spawnCharacter(
		config: MascotCharacterConfig_Serialize,
		x: number,
		y: number,
		fallbackAvatarPath?: string | null,
		callbacks?: CharacterCallbacks,
	): Promise<Character> {
		const char = new Character(config, callbacks);
		char.floorY = y;
		await char.init(fallbackAvatarPath);
		char.setPosition(x, y);
		this.characters.set(config.id, char);
		return char;
	}

	get(id: string): Character | undefined {
		return this.characters.get(id);
	}

	getAll(): Character[] {
		return Array.from(this.characters.values());
	}

	count(): number {
		return this.characters.size;
	}

	findNearestOther(id: string): Character | null {
		const self = this.characters.get(id);
		if (!self) return null;
		let best: Character | null = null;
		let bestDist = Infinity;
		for (const other of this.characters.values()) {
			if (other.id === id) continue;
			const dist = Math.abs(other.container.x - self.container.x);
			if (dist < bestDist) {
				bestDist = dist;
				best = other;
			}
		}
		return best;
	}

	update(deltaMS: number) {
		for (const character of this.characters.values()) {
			character.update(deltaMS);
		}
	}

	updateFloor(floorY: number) {
		for (const character of this.characters.values()) {
			const wasAtFloor = Math.abs(character.container.y - character.floorY) < 4;
			character.floorY = floorY;
			if (wasAtFloor || character.container.y > floorY) {
				character.container.y = floorY;
			}
		}
	}

	removeCharacter(id: string) {
		const character = this.characters.get(id);
		if (character) {
			character.destroy();
			this.characters.delete(id);
		}
	}

	clear() {
		for (const character of this.characters.values()) {
			character.destroy();
		}
		this.characters.clear();
	}
}
