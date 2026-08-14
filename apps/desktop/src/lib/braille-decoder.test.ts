import { describe, expect, test } from 'bun:test'

import { decodeBraille } from './braille-decoder'

describe('decodeBraille', () => {
  test('대표적인 한국 점자를 한글로 역점역한다', () => {
    expect(decodeBraille('⠣⠒⠉⠻')).toBe('안녕')
  })

  test('약자와 공백을 포함한 입력을 역점역한다', () => {
    expect(decodeBraille('⠣⠒⠀⠉⠻')).toBe('안 녕')
    expect(decodeBraille('⠠⠻⠨⠻⠰⠻')).toBe('성정청')
  })
})
