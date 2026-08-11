'use client'

import { Box, Flex, Text, VStack } from '@devup-ui/react'
import { useEffect, useRef, useState } from 'react'

import { type DotNumber, masksToString, toggleDot } from '../lib/braille'
import { BrailleKeyboard } from './BrailleKeyboard'
import { BrailleTextInput } from './BrailleTextInput'

interface BrailleInputProps {
  cells: number[]
  onChange: (cells: number[]) => void
}

// 브라유 유니코드 범위: U+2800 ~ U+283F
const BRAILLE_START = 0x2800
const BRAILLE_END = 0x283f

function parseToCells(raw: string): number[] {
  const result: number[] = []
  for (const ch of raw) {
    const code = ch.codePointAt(0) ?? 0
    if (code >= BRAILLE_START && code <= BRAILLE_END) {
      result.push(code - BRAILLE_START)
    } else if (ch === ' ' || ch === '\n') {
      result.push(0) // 일반 공백·줄바꿈 → ⠀(U+2800) 점자 공백으로 보존
    }
    // 그 외 문자는 무시
  }
  return result
}

export function BrailleInput({ cells, onChange }: BrailleInputProps) {
  // 각 셀에 안정적인 고유 key를 부여하기 위한 카운터 + id 배열
  // (배열 인덱스를 key로 쓰면 삭제 시 React가 잘못된 컴포넌트를 재사용할 수 있음)
  const counter = useRef(0)
  const nextId = () => counter.current++
  const [cellIds, setCellIds] = useState<number[]>([])

  // 외부에서 cells가 초기화될 때(방향 전환 등) ids도 동기화
  useEffect(() => {
    if (cells.length === 0) setCellIds([])
  }, [cells.length])

  // ── 텍스트 직접 입력 ──────────────────────────────────────────
  function handleTextChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const newCells = parseToCells(e.target.value)
    // 텍스트 붙여넣기로 전체 교체 → 모든 셀에 새 id 발급
    setCellIds(newCells.map(() => nextId()))
    onChange(newCells)
  }

  // ── 도트 키보드 ───────────────────────────────────────────────
  function handleToggleDot(cellIndex: number, dot: DotNumber) {
    // 점 토글은 셀 구조가 바뀌지 않으므로 id 변경 불필요
    onChange(cells.map((m, i) => (i === cellIndex ? toggleDot(m, dot) : m)))
  }

  function handleAddCell() {
    setCellIds((ids) => [...ids, nextId()])
    onChange([...cells, 0])
  }

  function handleAddSpace() {
    setCellIds((ids) => [...ids, nextId()])
    onChange([...cells, 0]) // mask 0 → ⠀ (점자 공백)
  }

  function handleBackspace() {
    if (cells.length > 0) {
      setCellIds((ids) => ids.slice(0, -1))
      onChange(cells.slice(0, -1))
    }
  }

  function handleRemoveCell(i: number) {
    setCellIds((ids) => ids.filter((_, idx) => idx !== i))
    onChange(cells.filter((_, idx) => idx !== i))
  }

  function handleClear() {
    setCellIds([])
    onChange([])
  }

  const unicodeValue = masksToString(cells)

  return (
    <VStack gap="14px">
      <BrailleTextInput onChange={handleTextChange} value={unicodeValue} />

      <Flex alignItems="center" gap="10px">
        <Box bg="$border" flex={1} h="1px" />
        <Text color="$textSubtle" flexShrink={0} typography="captionSm">
          또는 도트 키보드로 조합
        </Text>
        <Box bg="$border" flex={1} h="1px" />
      </Flex>

      <BrailleKeyboard
        cellIds={cellIds}
        cells={cells}
        onAddCell={handleAddCell}
        onAddSpace={handleAddSpace}
        onBackspace={handleBackspace}
        onClear={handleClear}
        onRemoveCell={handleRemoveCell}
        onToggleDot={handleToggleDot}
      />
    </VStack>
  )
}
