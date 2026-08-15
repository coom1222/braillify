export const BRAILLE_BLANK = '\u2800'

export function createBrailleCell(dots: readonly number[]): string {
  const bitMask = dots.reduce((mask, dot) => {
    if (!Number.isInteger(dot) || dot < 1 || dot > 6) {
      throw new Error(`유효하지 않은 점 번호입니다: ${dot}`)
    }

    return mask | (1 << (dot - 1))
  }, 0)

  return String.fromCodePoint(0x2800 + bitMask)
}

export function deleteLastCharacter(value: string): string {
  return Array.from(value).slice(0, -1).join('')
}
