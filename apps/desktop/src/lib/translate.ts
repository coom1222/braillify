import { invoke } from '@tauri-apps/api/core'

import type { TranslateMode } from '@/constants/translation'

export const EMPTY_INPUT_MESSAGE = '점역할 내용을 입력해 주세요.'
export const MATH_DELIMITER_MESSAGE =
  'LaTeX 수식 전체를 $...$ 형식으로 입력해 주세요.'
export const MATH_BRACE_MESSAGE = 'LaTeX 중괄호의 짝을 확인해 주세요.'
export const TRANSLATION_ERROR_MESSAGE =
  '이 내용은 점역할 수 없습니다. 입력을 확인해 주세요.'

type InvokeCommand = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<string>

const invokeTranslation: InvokeCommand = (command, args) =>
  invoke<string>(command, args)

export async function translateGeneralText(
  input: string,
  invokeCommand: InvokeCommand = invokeTranslation,
): Promise<string> {
  if (input.trim().length === 0) {
    throw new Error(EMPTY_INPUT_MESSAGE)
  }

  try {
    return await invokeCommand('translate_to_unicode', { input })
  } catch (error) {
    if (error instanceof Error && error.message === EMPTY_INPUT_MESSAGE) {
      throw error
    }

    throw new Error(TRANSLATION_ERROR_MESSAGE, { cause: error })
  }
}

export function validateMathInput(input: string): string {
  const normalizedInput = input.trim()

  if (
    normalizedInput.length < 3 ||
    !normalizedInput.startsWith('$') ||
    !normalizedInput.endsWith('$')
  ) {
    throw new Error(MATH_DELIMITER_MESSAGE)
  }

  let braceDepth = 0

  for (let index = 1; index < normalizedInput.length - 1; index += 1) {
    const character = normalizedInput[index]

    if (character === '\\') {
      index += 1
      continue
    }

    if (character === '$') {
      throw new Error(MATH_DELIMITER_MESSAGE)
    }

    if (character === '{') {
      braceDepth += 1
    } else if (character === '}') {
      braceDepth -= 1
      if (braceDepth < 0) {
        throw new Error(MATH_BRACE_MESSAGE)
      }
    }
  }

  if (braceDepth !== 0) {
    throw new Error(MATH_BRACE_MESSAGE)
  }

  return normalizedInput
}

export async function translateText(
  input: string,
  mode: TranslateMode,
  invokeCommand: InvokeCommand = invokeTranslation,
): Promise<string> {
  if (mode === 'math') {
    return translateGeneralText(validateMathInput(input), invokeCommand)
  }

  return translateGeneralText(input, invokeCommand)
}
