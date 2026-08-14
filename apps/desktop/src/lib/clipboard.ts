import { isTauri } from '@tauri-apps/api/core'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

export const EMPTY_CLIPBOARD_MESSAGE = '복사할 점역 결과가 없습니다.'
export const CLIPBOARD_ERROR_MESSAGE =
  '결과를 복사하지 못했습니다. 다시 시도해 주세요.'

type WriteClipboardText = (text: string) => Promise<void>

const writeClipboardText: WriteClipboardText = async (text) => {
  if (isTauri()) {
    await writeText(text)
    return
  }

  const browserClipboard = globalThis.navigator?.clipboard

  if (!browserClipboard) {
    throw new Error('브라우저에서 클립보드를 사용할 수 없습니다.')
  }

  await browserClipboard.writeText(text)
}

export async function copyText(
  text: string,
  writeClipboard: WriteClipboardText = writeClipboardText,
): Promise<void> {
  if (text.length === 0) {
    throw new Error(EMPTY_CLIPBOARD_MESSAGE)
  }

  try {
    await writeClipboard(text)
  } catch (error) {
    throw new Error(CLIPBOARD_ERROR_MESSAGE, { cause: error })
  }
}
