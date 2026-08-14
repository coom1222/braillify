import { expect, test } from '@playwright/test'

import koreanRule15 from '../../../test_cases/korean/rule_15.json'

const GENERAL_CASE = koreanRule15.find(({ input }) => input === '안녕')

if (!GENERAL_CASE) {
  throw new Error('E2E에 필요한 공식 테스트 케이스를 찾을 수 없습니다.')
}

const BRAILLE_RESULT = GENERAL_CASE.unicode

test.beforeEach(async ({ page }) => {
  await page.addInitScript(
    ({ brailleResult }) => {
      const tauriWindow = window as typeof window & {
        __copiedText?: string
        __failClipboard?: boolean
        __translatedInputs?: string[]
        isTauri: boolean
        __TAURI_INTERNALS__: {
          invoke: (
            command: string,
            args?: Record<string, unknown>,
          ) => Promise<unknown>
        }
      }

      tauriWindow.isTauri = true
      tauriWindow.__TAURI_INTERNALS__ = {
        invoke: async (command, args) => {
          if (command === 'plugin:clipboard-manager|write_text') {
            if (tauriWindow.__failClipboard) {
              throw new Error('clipboard unavailable')
            }
            tauriWindow.__copiedText = args?.text as string
            return
          }
          if (command === 'decode_from_unicode') {
            if (args?.input === '⠣⠒⠉⠻') {
              return '안녕'
            }
            throw new Error('지원하지 않는 점자')
          }
          if (command !== 'translate_to_unicode') {
            throw new Error(`알 수 없는 명령: ${command}`)
          }
          if (args?.input === '😀') {
            throw new Error('지원하지 않는 문자')
          }
          tauriWindow.__translatedInputs = [
            ...(tauriWindow.__translatedInputs ?? []),
            args?.input as string,
          ]
          return brailleResult
        },
      }
    },
    { brailleResult: BRAILLE_RESULT },
  )
})

test('앱 셸과 연결된 메뉴 및 초기 상태를 표시한다', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'Braillify' })).toBeVisible()
  await expect(page.getByRole('heading', { name: '점역기' })).toBeVisible()
  await expect(page.getByText('한글 점역 시스템')).toBeVisible()
  await expect(
    page.getByText(
      '일반 텍스트와 LaTeX 수식을 점역하거나 한국 점자를 한글로 역점역합니다.',
    ),
  ).toBeVisible()
  await expect(page.getByText('점역 · 역점역')).toBeVisible()
  await expect(page.getByText('직접 점자 편집')).toBeVisible()
  await expect(page.getByText('최근 작업 · 즐겨찾기')).toBeVisible()
  await expect(page.getByText('braillify 오픈소스 엔진')).toBeVisible()
  await expect(page.getByText('2024 한국 점자 규정 기반')).toBeVisible()

  const input = page.getByLabel('점역할 일반 텍스트')
  await expect(input).toHaveAttribute(
    'placeholder',
    '점역할 한국어 문장이나 단어를 입력하세요.',
  )
  await expect(page.getByText('0자')).toBeVisible()
  await expect(page.getByText('Ctrl + Enter로 변환')).toBeVisible()
  await expect(page.getByRole('button', { name: '점역하기' })).toBeDisabled()
  await expect(
    page.getByText('텍스트를 입력하고 점역을 시작해보세요'),
  ).toBeVisible()

  await expect(page.getByRole('button', { name: '일반' })).toHaveAttribute(
    'aria-pressed',
    'true',
  )
  await expect(page.getByRole('button', { name: '수학' })).toBeEnabled()
  await expect(page.getByRole('button', { name: '역점역' })).toBeEnabled()
  await expect(page.getByRole('button', { name: /점자 편집기/ })).toBeEnabled()
  await expect(page.getByRole('button', { name: /히스토리/ })).toBeEnabled()
})

test('문자 수를 유니코드 문자 단위로 표시한다', async ({ page }) => {
  await page.goto('/')

  const input = page.getByLabel('점역할 일반 텍스트')
  await input.fill(`${GENERAL_CASE.input}😀`)

  await expect(page.getByText('3자')).toBeVisible()
  await expect(page.getByRole('button', { name: '점역하기' })).toBeEnabled()
})

test('일반 텍스트를 버튼으로 점역하고 결과를 선택한다', async ({ page }) => {
  await page.goto('/')

  await page.getByLabel('점역할 일반 텍스트').fill(GENERAL_CASE.input)
  await page.getByRole('button', { name: '점역하기' }).click()

  const result = page.getByLabel('점역 결과 텍스트')
  await expect(result).toHaveText(BRAILLE_RESULT)
  await result.selectText()
  await expect
    .poll(() => page.evaluate(() => window.getSelection()?.toString()))
    .toBe(BRAILLE_RESULT)
})

test('Ctrl+Enter로 점역하고 Ctrl+Shift+C로 복사한다', async ({ page }) => {
  await page.goto('/')

  const input = page.getByLabel('점역할 일반 텍스트')
  await input.fill(GENERAL_CASE.input)
  await input.press('Control+Enter')

  await expect(page.getByLabel('점역 결과 텍스트')).toHaveText(BRAILLE_RESULT)
  await page.keyboard.press('Control+Shift+C')
  await expect(page.getByText('결과를 클립보드에 복사했습니다.')).toBeVisible()
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __copiedText?: string
            }
          ).__copiedText,
      ),
    )
    .toBe(BRAILLE_RESULT)
})

test('결과 복사 버튼과 실패 상태가 같은 결과 영역에 표시된다', async ({
  page,
}) => {
  await page.goto('/')

  await page.getByLabel('점역할 일반 텍스트').fill(GENERAL_CASE.input)
  await page.getByRole('button', { name: '점역하기' }).click()
  await page.evaluate(() => {
    ;(
      window as typeof window & {
        __failClipboard?: boolean
      }
    ).__failClipboard = true
  })
  await page.getByRole('button', { name: '결과 복사' }).click()

  await expect(
    page.getByRole('alert').filter({
      hasText: '결과를 복사하지 못했습니다. 다시 시도해 주세요.',
    }),
  ).toBeVisible()
})

test('빈 문자열과 공백 입력은 명령을 호출하지 않고 안내한다', async ({
  page,
}) => {
  await page.goto('/')

  const input = page.getByLabel('점역할 일반 텍스트')
  const emptyInputAlert = page
    .getByRole('alert')
    .filter({ hasText: '점역할 내용을 입력해 주세요.' })
  await input.focus()
  await input.press('Control+Enter')
  await expect(emptyInputAlert).toHaveText('점역할 내용을 입력해 주세요.')

  await input.fill(' \n\t')
  await input.press('Control+Enter')
  await expect(emptyInputAlert).toHaveText('점역할 내용을 입력해 주세요.')
  await expect(page.getByRole('button', { name: '점역하기' })).toBeDisabled()
})

test('지원하지 않는 입력은 결과 대신 오류를 안내한다', async ({ page }) => {
  await page.goto('/')

  await page.getByLabel('점역할 일반 텍스트').fill('😀')
  await page.getByRole('button', { name: '점역하기' }).click()

  await expect(
    page.getByRole('alert').filter({
      hasText: '이 내용은 점역할 수 없습니다. 입력을 확인해 주세요.',
    }),
  ).toHaveText('이 내용은 점역할 수 없습니다. 입력을 확인해 주세요.')
  await expect(page.getByLabel('점역 결과 텍스트')).toHaveCount(0)
  await expect(
    page.getByText('텍스트를 입력하고 점역을 시작해보세요'),
  ).toBeVisible()
})

test('줄바꿈과 긴 입력도 점역 흐름과 글자 수를 유지한다', async ({ page }) => {
  await page.goto('/')

  const input = page.getByLabel('점역할 일반 텍스트')
  const lineBreakInput = `${GENERAL_CASE.input}\n${GENERAL_CASE.input}`
  await input.fill(lineBreakInput)
  await input.press('Control+Enter')
  await expect(page.getByLabel('점역 결과 텍스트')).toHaveText(BRAILLE_RESULT)

  const longInput = GENERAL_CASE.input.repeat(2_000)
  await input.fill(longInput)
  await expect(page.getByText('4000자')).toBeVisible()
  await input.press('Control+Enter')
  await expect(input).toHaveValue(longInput)
  await expect(page.getByLabel('점역 결과 텍스트')).toHaveText(BRAILLE_RESULT)
})

test('최소 지원 창 크기에서 가로 스크롤이 생기지 않는다', async ({ page }) => {
  await page.setViewportSize({ height: 640, width: 960 })
  await page.goto('/')

  await expect(page.getByLabel('점역할 일반 텍스트')).toBeVisible()
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true)
})

test('사이드바로 점역기, 점자 편집기, 히스토리를 전환한다', async ({
  page,
}) => {
  await page.goto('/')

  await page.getByRole('button', { name: /점자 편집기/ }).click()
  await expect(page.getByRole('heading', { name: '점자 편집기' })).toBeVisible()
  await expect(page.getByLabel('점자 점 선택')).toBeVisible()

  await page.getByRole('button', { name: /히스토리/ }).click()
  await expect(page.getByRole('heading', { name: '히스토리' })).toBeVisible()
  await expect(
    page.getByText('아직 저장된 점역 기록이 없습니다.'),
  ).toBeVisible()

  await page.getByRole('button', { name: /점역기/ }).click()
  await expect(page.getByRole('heading', { name: '점역기' })).toBeVisible()
  await expect(page.getByLabel('점역할 일반 텍스트')).toBeVisible()
})

test('수학 모드를 선택하고 LaTeX 검증 후 점역한다', async ({ page }) => {
  await page.goto('/')

  await page.getByRole('button', { name: '수학' }).click()
  const mathInput = page.getByLabel('점역할 LaTeX 수식')
  await expect(mathInput).toHaveAttribute('placeholder', '$\\frac{3}{4}$')

  await mathInput.fill('x+1')
  await mathInput.press('Control+Enter')
  await expect(
    page.getByRole('alert').filter({
      hasText: 'LaTeX 수식 전체를 $...$ 형식으로 입력해 주세요.',
    }),
  ).toBeVisible()

  await mathInput.fill('$\\frac{3}{4}$')
  await page.getByRole('button', { name: '수학 점역하기' }).click()
  await expect(page.getByLabel('점역 결과 텍스트')).toHaveText(BRAILLE_RESULT)
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __translatedInputs?: string[]
            }
          ).__translatedInputs,
      ),
    )
    .toContain('$\\frac{3}{4}$')
})

test('역점역 모드에서 직접 입력과 6점 입력기로 한글 결과를 만든다', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: '역점역' }).click()

  const input = page.getByLabel('역점역할 점자')
  await expect(input).toHaveAttribute('placeholder', '예: ⠣⠒⠉⠻')
  await expect(
    page.getByText('점자를 입력하고 역점역을 시작해보세요'),
  ).toBeVisible()

  await page.getByRole('button', { name: '역점역 점 1' }).click()
  await page.getByRole('button', { name: '역점역 점 2' }).click()
  await expect(page.getByLabel('역점역 현재 점자 셀')).toHaveText('⠃')
  await page.getByRole('button', { name: '셀 추가' }).click()
  await expect(input).toHaveValue('⠃')
  await page.getByRole('button', { name: '전체 지우기' }).click()

  await input.fill('⠣⠒⠉⠻')
  await page.getByRole('button', { name: '역점역하기' }).click()
  await expect(page.getByLabel('역점역 결과 텍스트')).toHaveText('안녕')

  await page.getByRole('button', { name: '결과 복사' }).click()
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __copiedText?: string
            }
          ).__copiedText,
      ),
    )
    .toBe('안녕')
})

test('6점 조합으로 점자 셀을 편집하고 복사한다', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: /점자 편집기/ }).click()

  await page.getByRole('button', { name: '점 1' }).click()
  await page.getByRole('button', { name: '점 4' }).click()
  await expect(page.getByLabel('현재 점자 셀')).toHaveText('⠉')
  await page.getByRole('button', { name: '셀 추가' }).click()
  await expect(page.getByLabel('점자 편집 내용')).toHaveValue('⠉')

  await page.getByRole('button', { name: '띄어쓰기' }).click()
  await expect(page.getByLabel('점자 편집 내용')).toHaveValue('⠉ ')
  await page.getByRole('button', { name: '한 칸 삭제' }).click()
  await expect(page.getByLabel('점자 편집 내용')).toHaveValue('⠉')

  await page.getByRole('button', { name: '편집 내용 복사' }).click()
  await expect(
    page.getByText('편집한 점자를 클립보드에 복사했습니다.'),
  ).toBeVisible()
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as typeof window & {
              __copiedText?: string
            }
          ).__copiedText,
      ),
    )
    .toBe('⠉')
})

test('점역 기록을 즐겨찾기하고 점역기로 다시 불러온다', async ({ page }) => {
  await page.goto('/')
  await page.getByLabel('점역할 일반 텍스트').fill(GENERAL_CASE.input)
  await page.getByRole('button', { name: '점역하기' }).click()

  await page.getByRole('button', { name: /히스토리/ }).click()
  await expect(
    page.getByRole('article').getByText(GENERAL_CASE.input, { exact: true }),
  ).toBeVisible()
  await expect(page.getByLabel('저장된 점역 결과')).toHaveText(BRAILLE_RESULT)

  await page
    .getByRole('button', { name: `즐겨찾기 추가: ${GENERAL_CASE.input}` })
    .click()
  await expect(
    page.getByRole('button', { name: `즐겨찾기 해제: ${GENERAL_CASE.input}` }),
  ).toBeVisible()
  await page.getByRole('button', { name: '즐겨찾기 1' }).click()
  await expect(
    page.getByRole('article').getByText(GENERAL_CASE.input, { exact: true }),
  ).toBeVisible()

  await page.getByRole('button', { name: '점역기로 불러오기' }).click()
  await expect(page.getByRole('heading', { name: '점역기' })).toBeVisible()
  await expect(page.getByLabel('점역할 일반 텍스트')).toHaveValue(
    GENERAL_CASE.input,
  )
  await expect(page.getByLabel('점역 결과 텍스트')).toHaveText(BRAILLE_RESULT)
})
