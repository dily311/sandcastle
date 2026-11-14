interface GenerationTask {
	id: string;
	conversationId: string;
	assistantMessageId: string;
	status: "pending" | "running" | "completed" | "failed" | "cancelled" | "interrupted";
}

interface GenerationStore {
	tasks: Map<string, GenerationTask>;

	startGeneration: (conversationId: string, message: string) => Promise<void>;
	cancelGeneration: (taskId: string) => Promise<void>;
	loadActiveTasks: () => Promise<void>;
	syncTaskState: () => Promise<void>;
}

// 작업 중단 시 흐름
// Notification Action
//        ↓
// GenerationActionReceiver
//        ↓
// JNI / Native Bridge
//        ↓
// Rust TaskManager.cancel(task_id)
//        ↓
// CancellationToken
//        ↓
// HTTP Stream 중단
//        ↓
// SQLite 상태 변경