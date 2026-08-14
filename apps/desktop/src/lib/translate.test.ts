import { describe, expect, mock, test } from 'bun:test'

import {
  EMPTY_INPUT_MESSAGE,
  EMPTY_REVERSE_INPUT_MESSAGE,
  INVALID_BRAILLE_MESSAGE,
  MATH_BRACE_MESSAGE,
  MATH_DELIMITER_MESSAGE,
  translateGeneralText,
  translateReverseText,
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

  test('브라우저에서는 WASM 어댑터로 정상 점역한다', async () => {
    const translateToUnicode = mock(() => '⠣⠒⠉⠻')

    mock.module('braillify', () => ({
      decodeFromUnicode: () => '안녕',
      translateToUnicode,
    }))

    await expect(translateGeneralText('안녕')).resolves.toBe('⠣⠒⠉⠻')

    expect(translateToUnicode).toHaveBeenCalledWith('안녕')
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

describe('translateText reverse mode', () => {
  test('점자 유니코드를 역점역 명령에 전달한다', async () => {
    const invokeCommand = mock(async () => '안녕')

    await expect(translateReverseText('  ⠣⠒⠉⠻  ', invokeCommand)).resolves.toBe(
      '안녕',
    )
    expect(invokeCommand).toHaveBeenCalledWith('decode_from_unicode', {
      input: '⠣⠒⠉⠻',
    })
  })

  test('브라우저 개발환경에서도 점자를 한글로 역점역한다', async () => {
    await expect(translateText('⠣⠒⠉⠻', 'reverse')).resolves.toBe('안녕')
  })

  test('빈 입력과 일반 문자가 섞인 입력을 거절한다', async () => {
    const invokeCommand = mock(async () => '호출되지 않아야 함')

    await expect(translateReverseText('   ', invokeCommand)).rejects.toThrow(
      EMPTY_REVERSE_INPUT_MESSAGE,
    )
    await expect(translateReverseText('⠣안녕', invokeCommand)).rejects.toThrow(
      INVALID_BRAILLE_MESSAGE,
    )
    await expect(translateReverseText('⡀', invokeCommand)).rejects.toThrow(
      INVALID_BRAILLE_MESSAGE,
    )
    expect(invokeCommand).not.toHaveBeenCalled()
  })
})
