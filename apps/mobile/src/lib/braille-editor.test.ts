import { describe, expect, test } from 'bun:test'

import {
  BRAILLE_BLANK,
  createBrailleCell,
  deleteLastCharacter,
} from './braille-editor'

describe('braille editor helpers', () => {
  test('선택한 점을 유니코드 점자 셀로 조합한다', () => {
    expect(createBrailleCell([])).toBe(BRAILLE_BLANK)
    expect(createBrailleCell([1])).toBe('⠁')
    expect(createBrailleCell([1, 4])).toBe('⠉')
    expect(createBrailleCell([1, 2, 3, 4, 5, 6])).toBe('⠿')
  })

  test('중복 점은 한 번만 반영하고 잘못된 점 번호는 거절한다', () => {
    expect(createBrailleCell([1, 1, 4])).toBe('⠉')
    expect(() => createBrailleCell([0])).toThrow('유효하지 않은 점 번호')
    expect(() => createBrailleCell([7])).toThrow('유효하지 않은 점 번호')
    expect(() => createBrailleCell([9])).toThrow('유효하지 않은 점 번호')
  })

  test('유니코드 문자 단위로 마지막 글자를 지운다', () => {
    expect(deleteLastCharacter('⠁⠉')).toBe('⠁')
    expect(deleteLastCharacter('⠁😀')).toBe('⠁')
    expect(deleteLastCharacter('')).toBe('')
  })
})
