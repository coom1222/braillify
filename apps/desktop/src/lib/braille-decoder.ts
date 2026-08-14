type Stage =
  | { kind: 'start' }
  | { choseong: number; kind: 'choseong' }
  | { choseong: number; jungseong: number; kind: 'jungseong' }

const START_STAGE: Stage = { kind: 'start' }

export function decodeBraille(braille: string): string {
  const cells: number[] = []

  for (const character of braille) {
    const code = character.codePointAt(0) ?? 0
    if (code >= 0x2800 && code <= 0x28ff) {
      cells.push(code - 0x2800)
    } else if (character === ' ' || character === '\n') {
      cells.push(0)
    }
  }

  let result = ''
  let index = 0
  let stage: Stage = START_STAGE

  while (index < cells.length) {
    const cell = cells[index] ?? 0

    if (stage.kind === 'start') {
      const shortcut = twoCellShortcut(cells, index)
      const abbreviated = syllableShortcut(cell)
      const consonantA = consonantAShortcut(cell)
      const vowel = jungseong(cells, index)
      const initial = choseong(cells, index)

      if (cell === 0) {
        result += ' '
        index += 1
      } else if (shortcut) {
        result += shortcut[0]
        index += shortcut[1]
      } else if (abbreviated) {
        result += buildSyllable(11, abbreviated[0], abbreviated[1])
        index += 1
      } else if (consonantA !== undefined) {
        stage = { choseong: consonantA, jungseong: 0, kind: 'jungseong' }
        index += 1
      } else if (vowel) {
        stage = { choseong: 11, jungseong: vowel[0], kind: 'jungseong' }
        index += vowel[1]
      } else if (initial) {
        stage = { choseong: initial[0], kind: 'choseong' }
        index += initial[1]
      } else {
        index += 1
      }
      continue
    }

    if (stage.kind === 'choseong') {
      const vowel = jungseong(cells, index)
      const abbreviated = syllableShortcut(cell)

      if (vowel) {
        stage = {
          choseong: stage.choseong,
          jungseong: vowel[0],
          kind: 'jungseong',
        }
        index += vowel[1]
      } else if (abbreviated) {
        result += buildSyllable(stage.choseong, abbreviated[0], abbreviated[1])
        stage = START_STAGE
        index += 1
      } else if (cell === 0) {
        result += `${choseongJamo(stage.choseong)} `
        stage = START_STAGE
        index += 1
      } else {
        stage = {
          choseong: stage.choseong,
          jungseong: 0,
          kind: 'jungseong',
        }
      }
      continue
    }

    const shortcut = twoCellShortcut(cells, index)
    const final = jongseong(cells, index)
    const abbreviated = syllableShortcut(cell)
    const consonantA = consonantAShortcut(cell)
    const nextInitial = choseong(cells, index)
    const nextVowel = jungseong(cells, index)

    if (shortcut) {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      result += shortcut[0]
      stage = START_STAGE
      index += shortcut[1]
    } else if (final[0] > 0) {
      result += buildSyllable(stage.choseong, stage.jungseong, final[0])
      stage = START_STAGE
      index += final[1]
    } else if (cell === 0) {
      result += `${buildSyllable(stage.choseong, stage.jungseong, 0)} `
      stage = START_STAGE
      index += 1
    } else if (abbreviated) {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      result += buildSyllable(11, abbreviated[0], abbreviated[1])
      stage = START_STAGE
      index += 1
    } else if (consonantA !== undefined) {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      stage = { choseong: consonantA, jungseong: 0, kind: 'jungseong' }
      index += 1
    } else if (nextInitial) {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      stage = { choseong: nextInitial[0], kind: 'choseong' }
      index += nextInitial[1]
    } else if (nextVowel) {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      stage = {
        choseong: 11,
        jungseong: nextVowel[0],
        kind: 'jungseong',
      }
      index += nextVowel[1]
    } else {
      result += buildSyllable(stage.choseong, stage.jungseong, 0)
      stage = START_STAGE
      index += 1
    }
  }

  if (stage.kind === 'choseong') {
    result += buildSyllable(stage.choseong, 0, 0)
  } else if (stage.kind === 'jungseong') {
    result += buildSyllable(stage.choseong, stage.jungseong, 0)
  }

  return result.trim()
}

function twoCellShortcut(
  cells: number[],
  index: number,
): [string, number] | null {
  const pair = `${cells[index]},${cells[index + 1]}`
  const character = {
    '32,59': '성',
    '40,59': '정',
    '48,59': '청',
    '56,14': '것',
  }[pair]

  return character ? [character, 2] : null
}

function syllableShortcut(cell: number): [number, number] | null {
  return (
    (
      {
        27: [13, 4],
        30: [4, 8],
        31: [20, 4],
        33: [6, 4],
        45: [8, 1],
        46: [18, 8],
        47: [13, 8],
        51: [6, 8],
        53: [18, 4],
        55: [8, 4],
        57: [4, 1],
        59: [6, 21],
        62: [4, 4],
        63: [8, 21],
      } as Record<number, [number, number]>
    )[cell] ?? null
  )
}

function consonantAShortcut(cell: number): number | undefined {
  return ({ 7: 9, 43: 0 } as Record<number, number>)[cell]
}

function singleChoseong(cell: number): number | undefined {
  return (
    {
      8: 0,
      9: 2,
      10: 3,
      11: 15,
      16: 5,
      17: 6,
      19: 16,
      24: 7,
      25: 17,
      26: 18,
      32: 9,
      40: 12,
      48: 14,
    } as Record<number, number>
  )[cell]
}

function choseong(cells: number[], index: number): [number, number] | null {
  const first = cells[index]
  if (first === undefined) return null

  if (first === 32) {
    const tense = (
      {
        7: 10,
        8: 1,
        10: 4,
        24: 8,
        32: 10,
        40: 13,
        43: 1,
      } as Record<number, number>
    )[cells[index + 1] ?? -1]
    if (tense !== undefined) return [tense, 2]
  }

  const value = singleChoseong(first)
  return value === undefined ? null : [value, 1]
}

function jungseong(cells: number[], index: number): [number, number] | null {
  const pair = `${cells[index]},${cells[index + 1]}`
  const compound = (
    {
      '13,23': 16,
      '15,23': 15,
      '28,23': 3,
      '39,23': 10,
    } as Record<string, number>
  )[pair]
  if (compound !== undefined) return [compound, 2]

  const value = (
    {
      12: 7,
      13: 13,
      14: 4,
      15: 14,
      21: 20,
      23: 1,
      28: 2,
      29: 5,
      35: 0,
      37: 8,
      39: 9,
      41: 17,
      42: 18,
      44: 12,
      49: 6,
      58: 19,
      61: 11,
    } as Record<number, number>
  )[cells[index] ?? -1]
  return value === undefined ? null : [value, 1]
}

function jongseong(cells: number[], index: number): [number, number] {
  const compound = (
    {
      '1,1': 2,
      '1,4': 3,
      '18,5': 5,
      '18,52': 6,
      '2,1': 9,
      '2,3': 11,
      '2,4': 12,
      '2,34': 10,
      '2,38': 13,
      '2,50': 14,
      '2,52': 15,
      '3,4': 18,
    } as Record<string, number>
  )[`${cells[index]},${cells[index + 1]}`]
  if (compound !== undefined) return [compound, 2]

  const value = (
    {
      1: 1,
      2: 8,
      3: 17,
      4: 19,
      5: 22,
      6: 23,
      12: 20,
      18: 4,
      20: 7,
      22: 24,
      34: 16,
      38: 25,
      50: 26,
      52: 27,
      54: 21,
    } as Record<number, number>
  )[cells[index] ?? -1]
  return value === undefined ? [0, 0] : [value, 1]
}

function buildSyllable(
  choseong: number,
  jungseong: number,
  jongseong: number,
): string {
  return String.fromCodePoint(
    (choseong * 21 + jungseong) * 28 + jongseong + 0xac00,
  )
}

function choseongJamo(index: number): string {
  return Array.from('ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ')[index] ?? '?'
}
