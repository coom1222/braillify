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
      alignItems={['flex-start', null, 'center']}
      bg="$background"
      borderTop="1px solid $border"
      flexWrap="wrap"
      gap={['12px', null, '20px']}
      justifyContent="space-between"
      px="24px"
      py="18px"
    >
      <VStack
        flex={['1 1 0', null, '0 0 auto']}
        gap="6px"
        minW={['0', null, '148px']}
      >
        <Text typography="inputTitle">6점 입력기</Text>
        <Text color="$caption" typography="sidebarCaption">
          점을 고른 뒤 셀을 추가하세요.
        </Text>
      </VStack>

      <Grid
        aria-label="역점역 점자 점 선택"
        as="fieldset"
        border="none"
        flexShrink="0"
        gap="8px"
        gridTemplateColumns="repeat(2, 42px)"
        m="0"
        order={[2, null, 0]}
        p="0"
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

      <VStack alignItems="center" flexShrink="0" gap="6px" order={[1, null, 0]}>
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

      <Flex
        flex={['1 0 100%', null, '0 1 auto']}
        flexWrap={['nowrap', null, 'wrap']}
        gap={['6px', null, '8px']}
        justifyContent="flex-end"
        maxW={['100%', null, '300px']}
        minW="0"
        order={[3, null, 0]}
        w={['100%', null, 'auto']}
      >
        <Button
          bg="$primary"
          border="none"
          borderRadius="9px"
          color="$base"
          cursor="pointer"
          flex={['1 1 0', null, '0 0 auto']}
          fontSize={['12px', null, '14px']}
          minW="0"
          onClick={() => {
            onChange(value + preview)
            setSelectedDots([])
          }}
          px={['4px', null, '14px']}
          py="9px"
          type="button"
          typography="button"
          whiteSpace="nowrap"
        >
          셀 추가
        </Button>
        <Button
          bg="$containerBackground"
          border="1px solid $border"
          borderRadius="9px"
          color="$text"
          cursor="pointer"
          flex={['1 1 0', null, '0 0 auto']}
          fontSize={['12px', null, '14px']}
          minW="0"
          onClick={() => onChange(value + ' ')}
          px={['4px', null, '12px']}
          py="9px"
          type="button"
          whiteSpace="nowrap"
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
          flex={['1 1 0', null, '0 0 auto']}
          fontSize={['12px', null, '14px']}
          minW="0"
          onClick={() => onChange(deleteLastCharacter(value))}
          px={['4px', null, '12px']}
          py="9px"
          type="button"
          whiteSpace="nowrap"
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
          flex={['1 1 0', null, '0 0 auto']}
          fontSize={['12px', null, '14px']}
          minW="0"
          onClick={() => {
            onChange('')
            setSelectedDots([])
          }}
          px={['4px', null, '12px']}
          py="9px"
          type="button"
          whiteSpace="nowrap"
        >
          전체 지우기
        </Button>
      </Flex>
    </Flex>
  )
}
