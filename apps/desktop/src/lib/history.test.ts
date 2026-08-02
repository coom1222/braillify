import { describe, expect, test } from 'bun:test'

import {
  addHistoryEntry,
  clearHistory,
  deleteHistoryEntry,
  HISTORY_STORAGE_KEY,
  loadHistory,
  MAX_HISTORY_ENTRIES,
} from './history'

class MemoryStorage {
  private values = new Map<string, string>()

  getItem(key: string) {
    return this.values.get(key) ?? null
  }

  removeItem(key: string) {
    this.values.delete(key)
  }

  setItem(key: string, value: string) {
    this.values.set(key, value)
  }
}

describe('translation history', () => {
  test('최근 변환을 모드와 함께 저장하고 다시 읽는다', () => {
    const storage = new MemoryStorage()

    addHistoryEntry(
      { input: '안녕', mode: 'general', result: '⠣⠒⠉⠻' },
      storage,
      () => 'entry-1',
      () => new Date('2026-07-27T12:00:00.000Z'),
    )

    expect(loadHistory(storage)).toEqual([
      {
        createdAt: '2026-07-27T12:00:00.000Z',
        id: 'entry-1',
        input: '안녕',
        mode: 'general',
        result: '⠣⠒⠉⠻',
      },
    ])
  })

  test('최대 개수를 넘으면 가장 오래된 항목을 제거한다', () => {
    const storage = new MemoryStorage()

    for (let index = 0; index <= MAX_HISTORY_ENTRIES; index += 1) {
      addHistoryEntry(
        {
          input: `입력 ${index}`,
          mode: 'general',
          result: `결과 ${index}`,
        },
        storage,
        () => `entry-${index}`,
      )
    }

    const entries = loadHistory(storage)
    expect(entries).toHaveLength(MAX_HISTORY_ENTRIES)
    expect(entries[0]?.id).toBe(`entry-${MAX_HISTORY_ENTRIES}`)
    expect(entries.at(-1)?.id).toBe('entry-1')
  })

  test('손상된 저장 데이터는 삭제하고 빈 상태로 복구한다', () => {
    const storage = new MemoryStorage()
    storage.setItem(HISTORY_STORAGE_KEY, '{invalid json')

    expect(loadHistory(storage)).toEqual([])
    expect(storage.getItem(HISTORY_STORAGE_KEY)).toBeNull()
  })

  test('저장소가 없는 서버 환경에서는 빈 상태를 반환한다', () => {
    expect(loadHistory()).toEqual([])
  })

  test('배열이 아니거나 유효하지 않은 항목은 기록으로 불러오지 않는다', () => {
    const storage = new MemoryStorage()
    storage.setItem(HISTORY_STORAGE_KEY, '{}')

    expect(loadHistory(storage)).toEqual([])
    expect(storage.getItem(HISTORY_STORAGE_KEY)).toBeNull()

    storage.setItem(HISTORY_STORAGE_KEY, '[null]')
    expect(loadHistory(storage)).toEqual([])
  })

  test('기본 생성기로 식별자와 시각을 기록한다', () => {
    const storage = new MemoryStorage()
    const [entry] = addHistoryEntry(
      { input: '안녕', mode: 'general', result: '⠣⠒⠉⠻' },
      storage,
    )

    expect(entry?.id).toBeString()
    expect(Number.isNaN(Date.parse(entry?.createdAt ?? ''))).toBe(false)
  })

  test('저장소 접근이 모두 실패해도 점역 기록 흐름을 중단하지 않는다', () => {
    const storage = {
      getItem() {
        throw new Error('storage unavailable')
      },
      removeItem() {
        throw new Error('storage unavailable')
      },
      setItem() {
        throw new Error('storage unavailable')
      },
    }

    expect(loadHistory(storage)).toEqual([])
    expect(
      addHistoryEntry(
        { input: '안녕', mode: 'general', result: '⠣⠒⠉⠻' },
        storage,
        () => 'offline-entry',
        () => new Date('2026-07-27T12:00:00.000Z'),
      ),
    ).toEqual([
      {
        createdAt: '2026-07-27T12:00:00.000Z',
        id: 'offline-entry',
        input: '안녕',
        mode: 'general',
        result: '⠣⠒⠉⠻',
      },
    ])
    expect(clearHistory(storage)).toEqual([])
  })

  test('개별 삭제와 전체 삭제를 저장소에 반영한다', () => {
    const storage = new MemoryStorage()
    addHistoryEntry(
      { input: '안녕', mode: 'general', result: '⠣⠒⠉⠻' },
      storage,
      () => 'general-entry',
    )
    addHistoryEntry(
      { input: '$x$', mode: 'math', result: '⠭' },
      storage,
      () => 'math-entry',
    )

    expect(deleteHistoryEntry('general-entry', storage)).toHaveLength(1)
    expect(clearHistory(storage)).toEqual([])
    expect(loadHistory(storage)).toEqual([])
  })
})
