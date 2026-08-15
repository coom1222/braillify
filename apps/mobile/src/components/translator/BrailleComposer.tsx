'use client'

import { Button, Flex, Grid, Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

import { createBrailleCell, deleteLastCharacter } from '@/lib/braille-editor'

const DOTS = [1, 4, 2, 5, 3, 6] as const

type BrailleComposerProps = {
  onChange: (value: string) => void
  value: string
}

export function BrailleComposer({ onChange, value }: BrailleComposerProps) {
  const [selectedDots, setSelectedDots] = useState<number[]>([])
  const preview = createBrailleCell(selectedDots)

  const toggleDot = (dot: number) => {
    setSelectedDots((current) =>
      current.includes(dot)
        ? current.filter((selectedDot) => selectedDot !== dot)
        : [...current, dot],
    )
  }

  return (
    <Flex
      alignItems="center"
      bg="$background"
      borderTop="1px solid $border"
      flexWrap="wrap"
      gap="20px"
      justifyContent="space-between"
      px="24px"
      py="18px"
    >
      <VStack gap="6px" minW="148px">
        <Text typography="inputTitle">6점 입력기</Text>
        <Text color="$caption" typography="sidebarCaption">
          점을 고른 뒤 셀을 추가하세요.
        </Text>
      </VStack>

      <Grid
        aria-label="역점역 점자 점 선택"
        as="fieldset"
        border="none"
        gap="8px"
        gridTemplateColumns="repeat(2, 42px)"
      >
        {DOTS.map((dot) => {
          const selected = selectedDots.includes(dot)
          return (
            <Button
              key={dot}
              aria-label={`역점역 점 ${dot}`}
              aria-pressed={selected}
              bg={selected ? '$primary' : '$containerBackground'}
              border="1px solid $border"
              borderRadius="999px"
              color={selected ? '$base' : '$text'}
              cursor="pointer"
              h="42px"
              onClick={() => toggleDot(dot)}
              p="0"
              type="button"
              w="42px"
            >
              {dot}
            </Button>
          )
        })}
      </Grid>

      <VStack alignItems="center" gap="6px">
        <Text color="$caption" typography="sidebarCaption">
          현재 셀
        </Text>
        <Text
          aria-label="역점역 현재 점자 셀"
          bg="$containerBackground"
          border="1px solid $border"
          borderRadius="10px"
          fontFamily="Segoe UI Symbol, sans-serif"
          fontSize="32px"
          h="52px"
          lineHeight="52px"
          textAlign="center"
          w="64px"
        >
          {preview}
        </Text>
      </VStack>

      <Flex flexWrap="wrap" gap="8px" justifyContent="flex-end" maxW="300px">
        <Button
          bg="$primary"
          border="none"
          borderRadius="9px"
          color="$base"
          cursor="pointer"
          onClick={() => {
            onChange(value + preview)
            setSelectedDots([])
          }}
          px="14px"
          py="9px"
          type="button"
          typography="button"
        >
          셀 추가
        </Button>
        <Button
          bg="$containerBackground"
          border="1px solid $border"
          borderRadius="9px"
          color="$text"
          cursor="pointer"
          onClick={() => onChange(value + ' ')}
          px="12px"
          py="9px"
          type="button"
        >
          띄어쓰기
        </Button>
        <Button
          bg="$containerBackground"
          border="1px solid $border"
          borderRadius="9px"
          color="$text"
          cursor={value ? 'pointer' : 'default'}
          disabled={!value}
          onClick={() => onChange(deleteLastCharacter(value))}
          px="12px"
          py="9px"
          type="button"
        >
          한 칸 삭제
        </Button>
        <Button
          bg="transparent"
          border="1px solid $border"
          borderRadius="9px"
          color="$error"
          cursor={value ? 'pointer' : 'default'}
          disabled={!value}
          onClick={() => {
            onChange('')
            setSelectedDots([])
          }}
          px="12px"
          py="9px"
          type="button"
        >
          전체 지우기
        </Button>
      </Flex>
    </Flex>
  )
}
