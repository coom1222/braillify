import { describe, expect, mock, test } from 'bun:test'

import {
  EMPTY_INPUT_MESSAGE,
  MATH_BRACE_MESSAGE,
  MATH_DELIMITER_MESSAGE,
  translateGeneralText,
  translateText,
  TRANSLATION_ERROR_MESSAGE,
} from './translate'

describe('translateGeneralText', () => {
  test('정상 입력과 명령 인자를 Rust 어댑터에 그대로 전달한다', async () => {
    const invokeCommand = mock(async () => '⠣⠒⠉⠻')

    await expect(translateGeneralText('안녕', invokeCommand)).resolves.toBe(
      '⠣⠒⠉⠻',
    )
    expect(invokeCommand).toHaveBeenCalledWith('translate_to_unicode', {
      input: '안녕',
    })
  })

  test('공백뿐인 입력은 Rust를 호출하지 않고 거절한다', async () => {
    const invokeCommand = mock(async () => '호출되지 않아야 함')

    await expect(translateGeneralText(' \n\t', invokeCommand)).rejects.toThrow(
      EMPTY_INPUT_MESSAGE,
    )
    expect(invokeCommand).not.toHaveBeenCalled()
  })

  test('Rust 오류를 사용자용 메시지로 변환한다', async () => {
    const invokeCommand = mock(async () => {
      throw new Error('지원하지 않는 문자: 😀')
    })

    await expect(translateGeneralText('😀', invokeCommand)).rejects.toThrow(
      TRANSLATION_ERROR_MESSAGE,
    )
  })

  test('Rust가 반환한 빈 입력 오류는 그대로 전달한다', async () => {
    const invokeCommand = mock(async () => {
      throw new Error(EMPTY_INPUT_MESSAGE)
    })

    await expect(translateGeneralText('안녕', invokeCommand)).rejects.toThrow(
      EMPTY_INPUT_MESSAGE,
    )
  })

  test('기본 Tauri 어댑터 오류도 사용자용 메시지로 변환한다', async () => {
    await expect(translateGeneralText('안녕')).rejects.toThrow(
      TRANSLATION_ERROR_MESSAGE,
    )
  })
})

describe('translateText math mode', () => {
  test('올바른 $...$ LaTeX를 정규화해 Rust에 전달한다', async () => {
    const invokeCommand = mock(async () => '⠼⠙⠌⠉')

    await expect(
      translateText('  $\\frac{3}{4}$  ', 'math', invokeCommand),
    ).resolves.toBe('⠼⠙⠌⠉')
    expect(invokeCommand).toHaveBeenCalledWith('translate_to_unicode', {
      input: '$\\frac{3}{4}$',
    })
  })

  test('$ 구분자가 누락된 수식을 거절한다', async () => {
    const invokeCommand = mock(async () => '호출되지 않아야 함')

    await expect(
      translateText('\\frac{3}{4}', 'math', invokeCommand),
    ).rejects.toThrow(MATH_DELIMITER_MESSAGE)
    expect(invokeCommand).not.toHaveBeenCalled()
  })

  test('중괄호 짝이 맞지 않는 수식을 거절한다', async () => {
    const invokeCommand = mock(async () => '호출되지 않아야 함')

    await expect(
      translateText('$\\frac{3}{4$', 'math', invokeCommand),
    ).rejects.toThrow(MATH_BRACE_MESSAGE)
    expect(invokeCommand).not.toHaveBeenCalled()
  })

  test('수식 내부의 $ 구분자와 닫는 중괄호 선행을 거절한다', async () => {
    const invokeCommand = mock(async () => '호출되지 않아야 함')

    await expect(translateText('$x$y$', 'math', invokeCommand)).rejects.toThrow(
      MATH_DELIMITER_MESSAGE,
    )
    await expect(translateText('$}x$', 'math', invokeCommand)).rejects.toThrow(
      MATH_BRACE_MESSAGE,
    )
    expect(invokeCommand).not.toHaveBeenCalled()
  })

  test('일반 모드는 수학 검증 없이 Rust 어댑터로 전달한다', async () => {
    const invokeCommand = mock(async () => '⠣⠒⠉⠻')

    await expect(translateText('안녕', 'general', invokeCommand)).resolves.toBe(
      '⠣⠒⠉⠻',
    )
    expect(invokeCommand).toHaveBeenCalledWith('translate_to_unicode', {
      input: '안녕',
    })
  })
})
