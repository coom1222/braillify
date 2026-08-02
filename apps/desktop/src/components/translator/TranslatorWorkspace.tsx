'use client'

import { VStack } from '@devup-ui/react'
import { useState } from 'react'

import { useTranslationHistory } from '@/hooks/useTranslationHistory'
import { copyText } from '@/lib/clipboard'
import { translateText } from '@/lib/translate'

import { TranslationInputCard } from './TranslationInputCard'
import { type CopyState, TranslationOutput } from './TranslationOutput'

export function TranslatorWorkspace() {
  const [input, setInput] = useState('')
  const [result, setResult] = useState('')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [isTranslating, setIsTranslating] = useState(false)
  const [copyState, setCopyState] = useState<CopyState>('idle')
  const { addEntry } = useTranslationHistory()

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
      const translatedResult = await translateText(input, 'general')
      setResult(translatedResult)
      setCopyState('idle')
      addEntry({
        input,
        mode: 'general',
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
      gap="0"
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
      <TranslationInputCard
        errorMessage={errorMessage}
        input={input}
        isTranslating={isTranslating}
        onChange={(value) => {
          setInput(value)
          setResult('')
          setErrorMessage(null)
          setCopyState('idle')
        }}
        onSubmit={() => void translate()}
      />
      <TranslationOutput
        copyState={copyState}
        onCopy={() => void copyResult()}
        result={result}
      />
    </VStack>
  )
}
