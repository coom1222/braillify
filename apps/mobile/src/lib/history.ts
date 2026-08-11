export type TranslateMode = 'general' | 'math'

export interface HistoryItem {
  id: string
  source: string
  braille: string
  mode: TranslateMode
  favorite: boolean
  createdAt: number
}

const KEY = 'braillify.history.v1'

type Listener = () => void
const listeners = new Set<Listener>()

function emit() {
  for (const fn of listeners) fn()
}

function read(): HistoryItem[] {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

function write(items: HistoryItem[]): void {
  localStorage.setItem(KEY, JSON.stringify(items))
  emit()
}

export function listHistory(): HistoryItem[] {
  return read()
}

export function subscribeHistory(fn: Listener): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

export function pushHistory(input: {
  source: string
  braille: string
  mode: TranslateMode
}): HistoryItem {
  const item: HistoryItem = {
    id: crypto.randomUUID(),
    source: input.source,
    braille: input.braille,
    mode: input.mode,
    favorite: false,
    createdAt: Date.now(),
  }

  const next = [item, ...read()]
  write(next)

  return item
}

export function toggleFavorite(id: string): void {
  const next = read().map((it) =>
    it.id === id ? { ...it, favorite: !it.favorite } : it,
  )
  write(next)
}

export function removeHistory(id: string): void {
  const next = read().filter((it) => it.id !== id)
  write(next)
}
