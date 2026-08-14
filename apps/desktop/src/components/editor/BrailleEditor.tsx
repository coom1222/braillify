'use client'

import { Button, Flex, Grid, Input, Text, VStack } from '@devup-ui/react'
import { useState } from 'react'

import { type CopyState } from '@/components/translator/TranslationOutput'
import { createBrailleCell, deleteLastCharacter } from '@/lib/braille-editor'
import { copyText } from '@/lib/clipboard'

const DOTS = [1, 4, 2, 5, 3, 6] as const

const COPY_STATUS = {
  copied: '편집한 점자를 클립보드에 복사했습니다.',
  error: '점자를 복사하지 못했습니다. 다시 시도해 주세요.',
  idle: '',
} as const satisfies Record<CopyState, string>

export function BrailleEditor() {
  const [content, setContent] = useState('')
  const [copyState, setCopyState] = useState<CopyState>('idle')
  const [selectedDots, setSelectedDots] = useState<number[]>([])
  const preview = createBrailleCell(selectedDots)

  const toggleDot = (dot: number) => {
    setSelectedDots((current) =>
      current.includes(dot)
        ? current.filter((selectedDot) => selectedDot !== dot)
        : [...current, dot],
    )
  }

  const appendCell = () => {
    setContent((current) => current + preview)
    setSelectedDots([])
    setCopyState('idle')
  }

  const copyContent = async () => {
    try {
      await copyText(content)
      setCopyState('copied')
    } catch {
      setCopyState('error')
    }
  }

  return (
    <Grid
      gap="24px"
      gridTemplateColumns="minmax(280px, 0.72fr) minmax(0, 1.28fr)"
      w="100%"
    >
      <VStack
        as="section"
        bg="$containerBackground"
        border="1px solid $border"
        borderRadius="20px"
        gap="22px"
        p="24px"
      >
        <VStack gap="6px">
          <Text as="h3" typography="inputTitle">
            6점 조합
          </Text>
          <Text color="$caption" typography="sidebarBody">
            점을 선택하고 셀 추가를 누르세요. 선택이 없으면 빈 점자 셀이
            추가됩니다.
          </Text>
        </VStack>

        <Grid
          aria-label="점자 점 선택"
          as="fieldset"
          border="none"
          gap="16px"
          gridTemplateColumns="repeat(2, 72px)"
          justifyContent="center"
        >
          {DOTS.map((dot) => {
            const isSelected = selectedDots.includes(dot)

            return (
              <Button
                key={dot}
                aria-label={`점 ${dot}`}
                aria-pressed={isSelected}
                bg={isSelected ? '$primary' : '$background'}
                border="2px solid $border"
                borderRadius="999px"
                color={isSelected ? '$base' : '$text'}
                cursor="pointer"
                fontSize="20px"
                h="72px"
                onClick={() => toggleDot(dot)}
                w="72px"
              >
                {dot}
              </Button>
            )
          })}
        </Grid>

        <VStack alignItems="center" gap="12px">
          <Text color="$caption" typography="sidebarBody">
            현재 셀
          </Text>
          <Text
            aria-label="현재 점자 셀"
            border="1px solid $border"
            borderRadius="14px"
            fontFamily="Segoe UI Symbol, sans-serif"
            fontSize="48px"
            h="78px"
            lineHeight="78px"
            textAlign="center"
            w="92px"
          >
            {preview}
          </Text>
          <Button
            bg="$primary"
            border="none"
            borderRadius="12px"
            color="$base"
            cursor="pointer"
            onClick={appendCell}
            px="24px"
            py="13px"
            typography="button"
          >
            셀 추가
          </Button>
        </VStack>
      </VStack>

      <VStack
        as="section"
        bg="$containerBackground"
        border="1px solid $border"
        borderRadius="20px"
        overflow="hidden"
      >
        <Flex
          alignItems="center"
          borderBottom="1px solid $border"
          justifyContent="space-between"
          minH="66px"
          px="24px"
        >
          <Text as="h3" typography="inputTitle">
            편집 내용
          </Text>
          <Text aria-live="polite" color="$caption" typography="body">
            {Array.from(content).length}칸
          </Text>
        </Flex>

        <Input
          aria-label="점자 편집 내용"
          as="textarea"
          bg="transparent"
          border="none"
          color="$text"
          fontFamily="var(--font-spoqa-han-sans-neo), Segoe UI Symbol, sans-serif"
          fontSize="20px"
          lineHeight="1.5"
          minH="292px"
          onChange={(event) => {
            setContent(event.target.value)
            setCopyState('idle')
          }}
          p="22px"
          placeholder="점자 셀을 조합하거나 유니코드 점자를 직접 입력하세요."
          resize="none"
          value={content}
          w="100%"
        />

        <Flex
          alignItems="center"
          borderTop="1px solid $border"
          flexWrap="wrap"
          gap="10px"
          justifyContent="flex-end"
          minH="88px"
          px="20px"
          py="14px"
        >
          <Button
            bg="$background"
            border="1px solid $border"
            borderRadius="10px"
            color="$text"
            cursor="pointer"
            onClick={() => {
              setContent((current) => current + ' ')
              setCopyState('idle')
            }}
            px="14px"
            py="10px"
          >
            띄어쓰기
          </Button>
          <Button
            bg="$background"
            border="1px solid $border"
            borderRadius="10px"
            color="$text"
            cursor="pointer"
            disabled={!content}
            onClick={() => {
              setContent((current) => deleteLastCharacter(current))
              setCopyState('idle')
            }}
            px="14px"
            py="10px"
          >
            한 칸 삭제
          </Button>
          <Button
            bg="$background"
            border="1px solid $border"
            borderRadius="10px"
            color="$error"
            cursor="pointer"
            disabled={!content}
            onClick={() => {
              setContent('')
              setCopyState('idle')
            }}
            px="14px"
            py="10px"
          >
            전체 지우기
          </Button>
          <Button
            bg={content ? '$primary' : '$disabledBackground'}
            border="none"
            borderRadius="10px"
            color={content ? '$base' : '$disabledText'}
            cursor={content ? 'pointer' : 'default'}
            disabled={!content}
            onClick={() => void copyContent()}
            px="16px"
            py="11px"
            typography="button"
          >
            편집 내용 복사
          </Button>
        </Flex>
        <Text
          color={copyState === 'error' ? '$error' : '$caption'}
          minH="30px"
          pb="12px"
          px="24px"
          role={copyState === 'error' ? 'alert' : 'status'}
          typography="sidebarBody"
        >
          {COPY_STATUS[copyState]}
        </Text>
      </VStack>
    </Grid>
  )
}
