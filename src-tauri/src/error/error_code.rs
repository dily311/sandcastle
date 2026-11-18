use serde::{Deserialize, Serialize};

// 에러 코드 정의
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCategory {
    Archive,
    NotFound,
    Validation,
    Database,
    AiProvider,
    Io,
    Window,
    Game,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // 공통
    InternalError,
    ResourceNotFound,
    InvalidInput,

    // Database
    DatabaseError,
    DatabaseLocked,

    // AI Provider
    AiProviderError,
    AiProviderRateLimit,
    AiProviderAuthFailed,
    AiProviderModelNotFound,
    AiProviderTimeout,
    AiProviderConnectionFailed,
    AiRequestCancelled,
    AiContextOverflow,         // 고정 프롬프트만으로 컨텍스트 한도를 넘음
    AiFeatureUnsupported,      // 이 연결(공급자·모델)이 임베딩·도구·구조화 출력을 지원하지 않음 (공급자를 부르지 않고 거절)
    AiStructuredOutputInvalid, // 응답이 JSON이 아니거나 스키마에 맞지 않음 (details: problems, raw)
    AiOutputTruncated,         // 출력 한도(max_tokens)에서 잘려 도구 호출·JSON이 끊김 — 실행하지 않음

    // Task
    TaskInvalidTransition, // 현재 상태에서 허용되지 않는 요청 (이미 끝난 작업 취소 등)
    TaskExecutorMissing,   // 해당 작업 종류의 실행기가 등록되지 않음
    TaskCancelled,
    TaskInterrupted, // 앱/서비스 종료로 중단

    // Budget
    BudgetExceeded, // 예산 한도를 넘어 공급자를 부르지 않음 (details: apiProfileId, metric, period, used, limit, canOverride)

    // Game
    GameInvalidState,
    GameSceneNotFound,
    GameEventFailed,
    GameDataInvalid,    // 진단 에러로 시작 불가(details에 진단 목록)
    GameNotOpened,      // open_game을 안 부름
    GameNoSession,      // 진행 중인 세션이 없음
    GameSessionChanged, // AI를 기다리는 동안 세션이 바뀜
    GameActionBlocked,  // 이동/대화/아이템 사용이 규칙상 불가(이유는 details.reason: conditionNotMet, sceneActive, notPresent 등)

    // Archive
    ArchiveInvalid,
    ArchiveCorrupted,
    ArchiveUnsafePath,

    // 기타
    IoError,
    WindowError,
}
