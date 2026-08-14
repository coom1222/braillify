import type { TranslateMode } from '@/constants/translation'

export const HISTORY_STORAGE_KEY = 'braillify.translation-history.v1'
export const MAX_HISTORY_ENTRIES = 20

export type HistoryEntry = {
  createdAt: string
  favorite: boolean
  id: string
  input: string
  mode: TranslateMode
  result: string
}

export type HistoryEntryDraft = Pick<HistoryEntry, 'input' | 'mode' | 'result'>

type HistoryStorage = Pick<Storage, 'getItem' | 'removeItem' | 'setItem'>

function getStorage(storage?: HistoryStorage): HistoryStorage | null {
  if (storage) {
    return storage
  }

  return typeof window === 'undefined' ? null : window.localStorage
}

function isHistoryEntry(
  value: unknown,
): value is Omit<HistoryEntry, 'favorite'> & { favorite?: boolean } {
  if (!value || typeof value !== 'object') {
    return false
  }

  const entry = value as Partial<HistoryEntry>

  return (
    typeof entry.id === 'string' &&
    typeof entry.createdAt === 'string' &&
    !Number.isNaN(Date.parse(entry.createdAt)) &&
    typeof entry.input === 'string' &&
    typeof entry.result === 'string' &&
    (entry.favorite === undefined || typeof entry.favorite === 'boolean') &&
    (entry.mode === 'general' ||
      entry.mode === 'math' ||
      entry.mode === 'reverse')
  )
}

export function loadHistory(storage?: HistoryStorage): HistoryEntry[] {
  const targetStorage = getStorage(storage)
  if (!targetStorage) {
    return []
  }

  try {
    const serializedHistory = targetStorage.getItem(HISTORY_STORAGE_KEY)
    if (!serializedHistory) {
      return []
    }

    const parsedHistory: unknown = JSON.parse(serializedHistory)
    if (!Array.isArray(parsedHistory)) {
      targetStorage.removeItem(HISTORY_STORAGE_KEY)
      return []
    }

    return parsedHistory
      .filter(isHistoryEntry)
      .map((entry) => ({ ...entry, favorite: entry.favorite ?? false }))
      .slice(0, MAX_HISTORY_ENTRIES)
  } catch {
    try {
      targetStorage.removeItem(HISTORY_STORAGE_KEY)
    } catch {
      // 저장소가 비활성화된 환경에서도 빈 히스토리로 안전하게 복구한다.
    }
    return []
  }
}

function persistHistory(
  entries: HistoryEntry[],
  storage?: HistoryStorage,
): void {
  try {
    getStorage(storage)?.setItem(HISTORY_STORAGE_KEY, JSON.stringify(entries))
  } catch {
    // 점역은 저장소 할당량 또는 브라우저 정책과 무관하게 계속 동작해야 한다.
  }
}

export function addHistoryEntry(
  draft: HistoryEntryDraft,
  storage?: HistoryStorage,
  createId: () => string = () => crypto.randomUUID(),
  now: () => Date = () => new Date(),
): HistoryEntry[] {
  const nextEntry: HistoryEntry = {
    ...draft,
    createdAt: now().toISOString(),
    favorite: false,
    id: createId(),
  }
  const nextEntries = [nextEntry, ...loadHistory(storage)].slice(
    0,
    MAX_HISTORY_ENTRIES,
  )

  persistHistory(nextEntries, storage)
  return nextEntries
}

export function deleteHistoryEntry(
  id: string,
  storage?: HistoryStorage,
): HistoryEntry[] {
  const nextEntries = loadHistory(storage).filter((entry) => entry.id !== id)

  persistHistory(nextEntries, storage)
  return nextEntries
}

export function toggleFavoriteHistoryEntry(
  id: string,
  storage?: HistoryStorage,
): HistoryEntry[] {
  const nextEntries = loadHistory(storage).map((entry) =>
    entry.id === id ? { ...entry, favorite: !entry.favorite } : entry,
  )

  persistHistory(nextEntries, storage)
  return nextEntries
}

export function clearHistory(storage?: HistoryStorage): HistoryEntry[] {
  try {
    getStorage(storage)?.removeItem(HISTORY_STORAGE_KEY)
  } catch {
    // 저장소 접근이 막힌 경우에도 UI 상태는 비울 수 있도록 빈 배열을 반환한다.
  }

  return []
}
