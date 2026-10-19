// ! rust로 넘어오는 데이터를 정의
// ! 혼동이 없도록 models/game.rs와 구조체명을 동일하게 할 것

// ---------------------------------------------------------
// 게임 번들 데이터
// ---------------------------------------------------------
export interface GameManifest {
    id: string, // 프로젝트 고유 ID
    title: string,
    description: string | null,
    version: string, // 게임버전: semver, 세이브 호환성 체크용
    templateId: string,
    apiVersion: string,     // 게임 엔진 버전
    startLocationId: string | null,
    coverImage: string | null,
    customCss: unknown, // 게임에만 적용될 css
    settingsJson: unknown, // 실행시 기본 값
    createdAt: number,
    updatedAt: number,
}

/** 게임 메타 데이터 */
export interface GameProject {
	id: string;
	title: string;
	description: string;
	templateId: string;
	apiVersion: string;
	coverImage: string | null;
	startSceneKey: string | null;
	startMapId: string | null;
	startLocationId: string | null;
	worldInfoIds: string[];
	customCss: string | null;
	settingJson: string;
	createdAt: number;
	updatedAt: number;
}

export interface GameStateDef {
	id: string;
	gameId: string;
	description: string | null;
	key: string;
	type: string;
	defaultValue: string;
	createdAt: number;
	updatedAt: number;
}

export interface GameChoice {
	id: string;
	sceneId: string;
	text: string;
	nextSceneId: string | null;
	effect: string | null;
	sortOrder: number;
}

export interface GameMaps {
	id: string;
	gameId: string;
	name: string;
	dangerLevel: number;
	defaultWeatherPoolId: string | null;
	settingsJson: unknown;
	createdAt: number;
	updatedAt: number;
}

export interface GameMapsZone {
	id: string;
	mapId: string;
	name: string;
	tags: string[];
	settingsJson: unknown;
	createdAt: number;
	updatedAt: number;
}

export interface GameLocations {
	id: string;
	zoneId: string;
	name: string;
	description: string | null;
	settingsJson: unknown;
}
export interface GameLocationConnections {
	fromId: string;
	toId: string;
	travelSeconds: number;
	condition: string | null;
	settingsJson: unknown;
}

export interface GameInteractables {
	id: string;
	locationId: string;
	name: string;
	eventId: string | null;
}

export interface GameTimePeriods {
	id: string;
	gameId: string;
	name: string;
	sortOrder: number;
	startMinute: number;
	endMinute: number;
}

export interface GameWeatherTypes {
	id: string;
	gameId: string;
	name: string;
	encounterRateModifier: number;
	visibilityModifier: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameWeatherTransitions {
	fromWeatherId: string;
	toWeatherId: string;
	mapId: string;
	probability: number;
}

export interface GameEvents {
	id: string;
	gameId: string;
	eventType: string;
	name: string;
	description: string;
	requiresNpc: boolean;
	configJson: { [key: string]: unknown };
	effectJson: { [key: string]: unknown };
	cooldownSeconds: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameEventConditions {
	id: string;
	eventId: string;
	conditionType: string;
	conditionValue: string | null;
}
export interface GameEventCharacterPool {
	eventId: string;
	gameCharacterId: string;
	spawnWeight: number;
}

export interface GameEncounterTables {
	id: string;
	mapId: string;
	zoneId: string | null;
	locationId: string | null;
	baseTriggerChance: number;
}

export interface GameEncounterEntries {
	id: string;
	encounterTableId: string;
	eventId: string;
	weight: number;
	timePeriodId: string | null;
	weatherId: string | null;
}

export interface GameScene {
	id: string;
	gameId: string;
	sceneName: string;
	description: string | null;
	sceneKey: string;
	settingsJson: string | null;
	isEnding: boolean;
	sortOrder: number;
	createdAt: number;
	updatedAt: number;
}

export interface GameEndings {
	id: string;
	gameId: string;
	endingKey: string;
	title: string;
	description: string;
}
export interface GameCharacters {
	id: string;
	gameId: string;
	characterId: string | null;
	name: string;
	avatar: string | null;
	description: string | null;
	firstMessage: string | null;
	exampleDialogue: string | null;
	systemPromptOverride: string | null;
	behaviorType: string | null;
	worldInfoIds: string[];
	tags: string | null;
	settingsJson: unknown;
	createdAt: number;
	updatedAt: number;
}

// ---------------------------------------------------------
// 런타임 시 게임 엔진에서 넘어오는 데이터
// ---------------------------------------------------------
export interface GameResponse {
	events: GameEvent[];
	state: RenderState;
}

export interface GameJournal {
	sessionId: string;
	seq: number;
	kind: string;
	payloadJson: any;
	gameTime: number;
	createdAt: number;
}
export interface GameConversations {
	id: string;
	gameId: string;
	sessionId: string;
	branchKey: string;
	participantsKey: string;
	title: string | null;
	createdAt: number;
	updatedAt: number;
}
export interface GameMessages {
	id: string;
	seq: number;
	conversationId: string;
	role: string;
	name: string | null;
	characterId: string | null;
	content: string;
	meta: any;
	createdAt: number;
	updatedAt: number;
}

export interface NpcReply {
	text: string;
	events: GameEvent[];
	state: RenderState;
}
// ---------------------------------------------------------
// 게임 스테이터스
// ---------------------------------------------------------
export interface RenderState {
	locationId: string | null;
	mapId: string;
	zoneId: string | null;
	gameTime: number;
	weatherID: string;
	timePeriodId: string;
	currentSceneId: string | null;
	// currentNodeId: string | null;
	activeNpcId: string | null;
	flags: any;
}

// ---------------------------------------------------------
// 게임 세이브
// ---------------------------------------------------------
export interface GamePlayerSessions {
	id: string;
	gameId: string;
	playerId: string;
	currentSceneId: string | null;
	currentMapId: string | null;
	currentZoneId: string | null;
	currentLocationId: string | null;
	currentWeatherId: string | null;
	gameTime: number;
	flagsJson: any;
	parentSessionId: string | null;
	pendingScenesJson: any;
	forkedFromSeq: number | null;
	headSeq: number;
	updatedAt: number;
}

export interface GameSave {
	id: string;
	gameId: string;
	sessionId: string;
	slotNumber: number;
	currentSceneId: string | null;
	currentMapId: string | null;
	currentZoneId: string | null;
	currentLocationId: string | null;
	gameTime: number;
	currentWeatherId: string | null;
	stateSnapshot: string | null;
	updatedAt: number;
}

export interface SaveReport {}

// ---------------------------------------------------------
// 게임 커맨드들
// ---------------------------------------------------------
export enum GameCommand {
	MoveTo,
	Interact,
	Talk,
	StopTalk,
	SetVariable,
	Choose,
	AdvanceTime,
	UseItem,
}

export enum GameEvent {
	GameStarted,
	LocationChanged,
	ZoneChanged,
	TimeAdvanceRequested,
	TimeAdvanced,
	WeatherChanged,
	EncounterTriggered,
	TriggerActivated,
	DialogueStarted,
	DialogueEnded,
	ChoicePresented,
	ChoiceMade,
	SceneChanged,
	VariableChanged,
	ItemAdded,
	ItemRemoved,
	GameEnded,
}

// ---------------------------------------------------------
// 게임 에디터용
// ---------------------------------------------------------

export interface PackageData {
	project: GameProjectInput;
	stateDefs: GameStateDefInput[];
	maps: GameMapsInput[];
	zones: GameMapsZoneInput[];
	locations: GameLocationsInput[];
	locationConnections: GameLocationConnectionsInput[];
	interactables: GameInteractablesInput[];
	timePeriods: GameTimePeriodsInput[];
	weatherTypes: GameWeatherTypesInput[];
	weatherTransitions: GameWeatherTransitionsInput[];
	events: GameEventsInput[];
	eventConditions: GameEventConditionsInput[];
	eventCharacterPool: GameEventCharacterPoolInput[];
	encounterTables: GameEncounterTablesInput[];
	encounterEntries: GameEncounterEntriesInput[];
	scenes: GameSceneInput[];
	endings: GameEndingsInput[];
	characters: GameCharactersInput[];
	worldInfoBooks: PackageWorldInfoBook[];
}

export interface GameBundleRow {
	game: GameProject;
	stateDefs: GameStateDef[];
	scenes: GameScene[];
	events: GameEvents[];
}

export interface GameProjectInput {
	id: string;
	title: string;
	description: string;
	templateId: string;
	apiVersion: string;
	coverImage: string | null;
	startSceneKey: string | null;
	startMapId: string | null;
	startLocationId: string | null;
	worldInfoIds: string[];
	customCss: string | null;
	settingJson: string;
	createdAt: number;
	updatedAt: number;
}

export interface GameStateDefInput {
	id: string;
	gameId: string;
	description: string | null;
	key: string;
	type: string;
	defaultValue: string;
	createdAt: number;
	updatedAt: number;
}
export interface GameMapsInput {
	id: string | null;
	gameId: string;
	name: string;
	dangerLevel: number;
	defaultWeatherPoolId: string | null;
	settingsJson: unknown;
}
export interface GameMapsZoneInput {
	id: string | null;
	mapId: string;
	name: string;
	tags: string[];
	settingsJson: unknown;
}

export interface GameLocationsInput {
	id: string | null;
	zoneId: string;
	name: string;
	description: string | null;
	settingsJson: unknown;
}
export interface GameLocationConnectionsInput {
	fromId: string;
	toId: string;
	travelSeconds: number;
	condition: string | null;
	settingsJson: unknown;
}
export interface GameInteractablesInput {
	id: string | null;
	locationId: string;
	name: string;
	eventId: string | null;
}
export interface GameTimePeriodsInput {
	id: string | null;
	gameId: string;
	name: string;
	sortOrder: number;
	startMinute: number;
	endMinute: number;
}
export interface GameWeatherTypesInput {
	id: string | null;
	gameId: string;
	name: string;
	encounterRateModifier: number;
	visibilityModifier: number;
}

export interface GameWeatherTransitionsInput {
	fromWeatherId: string;
	toWeatherId: string;
	mapId: string;
	probability: number;
}
export interface GameEventsInput {
	id: string;
	gameId: string;
	eventType: string;
	name: string;
	description: string;
	requiresNpc: boolean;
	configJson: { [key: string]: unknown };
	effectJson: { [key: string]: unknown };
	cooldownSeconds: number;
	createdAt: number;
	updatedAt: number;
}
export interface GameEventConditionsInput {
	id: string | null;
	eventId: string;
	conditionType: string;
	conditionValue: string | null;
}
export interface GameEventCharacterPoolInput {
	id: string | null;
	eventId: string;
	gameCharacterId: string;
	spawnWeight: number;
}
export interface GameEncounterTablesInput {
	id: string | null;
	mapId: string;
	zoneId: string | null;
	locationId: string | null;
	baseTriggerChance: number;
}
export interface GameEncounterEntriesInput {
	id: string | null;
	encounterTableId: string;
	eventId: string;
	weight: number;
	timePeriodId: string | null;
	weatherId: string | null;
}
export interface GameSceneInput {
	id: string;
	gameId: string;
	sceneName: string;
	description: string | null;
	sceneKey: string;
	settingsJson: string | null;
	isEnding: boolean;
	sortOrder: number;
	createdAt: number;
	updatedAt: number;
}
export interface GameEndingsInput {
	id: string | null;
	gameId: string;
	endingKey: string;
	title: string;
	description: string;
}

export interface GameCharactersInput {
	id: string | null;
	gameId: string;
	characterId: string | null;
	name: string;
	avatar: string | null;
	description: string | null;
	firstMessage: string | null;
	exampleDialogue: string | null;
	systemPromptOverride: string | null;
	behaviorType: string | null;
	worldInfoIds: string[];
	tags: string | null;
	settingsJson: unknown;
}

export interface PackageWorldInfoBook {
	id: string;
	book: WorldInfoBookInput;
	entries: WorldInfoEntryInput[];
}

interface WorldInfoBookInput {
	id: string | null;
	name: string;
	description: string | null;
}
interface WorldInfoEntryInput {
	id: string | null;
	bookId: string;
	keys: string[];
	secondaryKeys: string[];
	content: string;
	enabled: boolean;
	priority: number;
	constant: boolean;
}
