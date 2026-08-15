'use client'

import { Button, Flex, Text, VStack } from '@devup-ui/react'
import { useEffect, useState } from 'react'

import { useAppState } from '@/components/shell/AppShell'
import {
  MODE_CONTENT,
  MODE_OPTIONS,
  type TranslateMode,
} from '@/constants/translation'
import { copyText } from '@/lib/clipboard'
import { translateText } from '@/lib/translate'

import { TranslationInputCard } from './TranslationInputCard'
import { type CopyState, TranslationOutput } from './TranslationOutput'

export function TranslatorWorkspace() {
  const { addEntry, restoreDraft } = useAppState()
  const [mode, setMode] = useState<TranslateMode>('general')
  const [input, setInput] = useState('')
  const [result, setResult] = useState('')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isTranslating, setIsTranslating] = useState(false)
  const [copyState, setCopyState] = useState<CopyState>('idle')
  const content = MODE_CONTENT[mode]

  useEffect(() => {
    if (!restoreDraft) {
      return
    }

    setMode(restoreDraft.mode)
    setInput(restoreDraft.input)
    setResult(restoreDraft.result)
    setErrorMessage(null)
    setCopyState('idle')
  }, [restoreDraft])

  const copyResult = async () => {
    try {
      await copyText(result)
      setCopyState('copied')
    } catch {
      setCopyState('error')
    }
  }

  const translate = async () => {
    if (isTranslating) {
      return
    }

    setErrorMessage(null)
    setIsTranslating(true)

    try {
      const translatedResult = await translateText(input, mode)
      setResult(translatedResult)
      setCopyState('idle')
      addEntry({
        input,
        mode,
        result: translatedResult,
      })
    } catch (error) {
      setResult('')
      setErrorMessage(
        error instanceof Error ? error.message : '점역 중 오류가 발생했습니다.',
      )
    } finally {
      setIsTranslating(false)
    }
  }

  return (
    <VStack
      gap="24px"
      onKeyDown={(event) => {
        if (
          event.ctrlKey &&
          event.shiftKey &&
          event.key.toLowerCase() === 'c'
        ) {
          event.preventDefault()
          if (result) {
            void copyResult()
          }
        }
      }}
      w="100%"
    >
      <Flex
        alignItems="center"
        bg="$containerBackground"
        border="1px solid $border"
        borderRadius="16px"
        gap="18px"
        justifyContent="space-between"
        px="20px"
        py="16px"
      >
        <VStack gap="4px">
          <Text as="h3" typography="inputTitle">
            점역 모드
          </Text>
          <Text color="$caption" typography="sidebarBody">
            {content.guide}
          </Text>
        </VStack>
        <Flex aria-label="점역 모드" as="fieldset" border="none" gap="8px">
          {MODE_OPTIONS.map((option) => {
            const isSelected = option.value === mode

            return (
              <Button
                key={option.value}
                aria-pressed={isSelected}
                bg={isSelected ? '$primary' : '$background'}
                border="1px solid $border"
                borderRadius="10px"
                color={isSelected ? '$base' : '$text'}
                cursor="pointer"
                onClick={() => {
                  if (option.value === mode) {
                    return
                  }
                  setMode(option.value)
                  setInput('')
                  setResult('')
                  setErrorMessage(null)
                  setCopyState('idle')
                }}
                px="18px"
                py="10px"
                type="button"
                typography="button"
              >
                {option.label}
              </Button>
            )
          })}
        </Flex>
      </Flex>
      <TranslationInputCard
        buttonLabel={content.buttonLabel}
        errorMessage={errorMessage}
        helpText={
          mode === 'math'
            ? '수식은 $...$로 감싸세요 · Ctrl + Enter로 변환'
            : mode === 'reverse'
              ? '점자 유니코드를 붙여넣거나 6점 입력기를 사용하세요 · Ctrl + Enter로 변환'
              : 'Ctrl + Enter로 변환'
        }
        input={input}
        inputLabel={content.inputLabel}
        isReverse={mode === 'reverse'}
        isTranslating={isTranslating}
        onChange={(value) => {
          setInput(value)
          setResult('')
          setErrorMessage(null)
          setCopyState('idle')
        }}
        onSubmit={() => void translate()}
        placeholder={content.placeholder}
      />
      <TranslationOutput
        copyState={copyState}
        isReverse={mode === 'reverse'}
        onCopy={() => void copyResult()}
        result={result}
      />
    </VStack>
  )
}
