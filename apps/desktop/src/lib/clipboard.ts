import { writeText } from '@tauri-apps/plugin-clipboard-manager'

export const EMPTY_CLIPBOARD_MESSAGE = '복사할 점역 결과가 없습니다.'
export const CLIPBOARD_ERROR_MESSAGE =
  '결과를 복사하지 못했습니다. 다시 시도해 주세요.'

type WriteClipboardText = (text: string) => Promise<void>

export async function copyText(
  text: string,
  writeClipboardText: WriteClipboardText = writeText,
): Promise<void> {
  if (text.length === 0) {
    throw new Error(EMPTY_CLIPBOARD_MESSAGE)
  }

  try {
    await writeClipboardText(text)
  } catch (error) {
    throw new Error(CLIPBOARD_ERROR_MESSAGE, { cause: error })
  }
}
