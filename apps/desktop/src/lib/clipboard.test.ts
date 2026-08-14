import { describe, expect, mock, test } from 'bun:test'

import {
  CLIPBOARD_ERROR_MESSAGE,
  copyText,
  EMPTY_CLIPBOARD_MESSAGE,
} from './clipboard'

describe('copyText', () => {
  test('점역 결과를 클립보드 어댑터에 전달한다', async () => {
    const writeClipboardText = mock(async () => undefined)

    await expect(copyText('⠣⠒⠉⠻', writeClipboardText)).resolves.toBeUndefined()
    expect(writeClipboardText).toHaveBeenCalledWith('⠣⠒⠉⠻')
  })

  test('빈 결과는 클립보드에 쓰지 않는다', async () => {
    const writeClipboardText = mock(async () => undefined)

    await expect(copyText('', writeClipboardText)).rejects.toThrow(
      EMPTY_CLIPBOARD_MESSAGE,
    )
    expect(writeClipboardText).not.toHaveBeenCalled()
  })

  test('클립보드 어댑터 실패를 사용자용 오류로 변환한다', async () => {
    const writeClipboardText = mock(async () => {
      throw new Error('clipboard unavailable')
    })

    await expect(copyText('⠣⠒⠉⠻', writeClipboardText)).rejects.toThrow(
      CLIPBOARD_ERROR_MESSAGE,
    )
  })
})
