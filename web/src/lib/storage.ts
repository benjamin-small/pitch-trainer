export const STORAGE_KEY = 'pitch-trainer:v1';
const VERSION = 1;

function defaultStorage(): Storage | undefined {
  try {
    return globalThis.localStorage;
  } catch {
    return undefined;
  }
}

/** Returns saved engine state JSON, or null if missing, corrupt, or from another version. */
export function loadEngineState(
  storage: Pick<Storage, 'getItem'> | undefined = defaultStorage(),
): string | null {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (!raw) return null;
    const saved = JSON.parse(raw);
    if (saved?.version !== VERSION || typeof saved.engine !== 'string') return null;
    return saved.engine;
  } catch {
    return null;
  }
}

export function saveEngineState(
  engineJson: string,
  storage: Pick<Storage, 'setItem'> | undefined = defaultStorage(),
): void {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify({ version: VERSION, engine: engineJson }));
  } catch {
    // Storage full or blocked: progress just isn't saved this time.
  }
}
