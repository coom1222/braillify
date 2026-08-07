'use client'

import { css, Flex, Text, VStack } from '@devup-ui/react'
import type { MathfieldElement } from 'mathlive'
import { useEffect, useRef, useState } from 'react'

import { normalizeFracBraces } from './normalizeFracBraces'

declare global {
  namespace React.JSX {
    interface IntrinsicElements {
      'math-field': React.DetailedHTMLProps<
        React.HTMLAttributes<MathfieldElement>,
        MathfieldElement
      > & { 'math-virtual-keyboard-policy'?: 'auto' | 'manual' }
    }
  }
}

export function MathTransInput({
  latex,
  onLatexChange,
  placeholder,
}: {
  latex: string
  onLatexChange: (latex: string) => void
  placeholder: string
}) {
  const [ready, setReady] = useState(false)
  const fieldRef = useRef<MathfieldElement>(null)

  useEffect(() => {
    let cancelled = false
    import('mathlive').then(({ MathfieldElement }) => {
      if (cancelled) return
      MathfieldElement.fontsDirectory = '/mathlive/fonts'
      MathfieldElement.soundsDirectory = null
      setReady(true)
    })
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    const field = fieldRef.current
    if (!ready || !field) return
    const show = () => window.mathVirtualKeyboard.show()
    const hide = () => window.mathVirtualKeyboard.hide()
    field.addEventListener('focusin', show)
    field.addEventListener('focusout', hide)
    return () => {
      field.removeEventListener('focusin', show)
      field.removeEventListener('focusout', hide)
      window.mathVirtualKeyboard.hide()
    }
  }, [ready])

  return (
    // 바깥 Flex 는 padding 없는 flex 아이템으로, 출력측 TransInput 의 외곽
    // Flex(flex=1 h=100% w=100%)와 flex-basis 를 동일하게 맞춰 좌우 박스 너비를
    // 같게 한다. 실제 배경/여백은 안쪽 박스가 담당한다.
    <Flex flex="1" h="100%" w="100%">
      <VStack
        bg="$containerBackground"
        borderRadius={['16px', null, null, '30px']}
        cursor="text"
        flex="1"
        gap="12px"
        h="100%"
        minH="25dvh"
        onClick={() => fieldRef.current?.focus()}
        p={['16px', null, null, '40px']}
      >
        <Flex flex="1" flexDirection="column" gap="8px">
          {ready && (
            <math-field
              ref={fieldRef}
              math-virtual-keyboard-policy="manual"
              onInput={(e) =>
                onLatexChange(
                  normalizeFracBraces(
                    (e.target as MathfieldElement).getValue(
                      'latex-without-placeholders',
                    ),
                  ),
                )
              }
              className={css({
                background: 'transparent',
                border: 'none',
                display: 'block',
                fontSize: '28px',
                width: '100%',
              })}
            />
          )}
          {!latex && (
            <Text
              color="$text"
              opacity={0.5}
              pointerEvents="none"
              typography="braille"
              whiteSpace="pre-line"
            >
              {placeholder}
            </Text>
          )}
        </Flex>
        <Text
          color="$text"
          fontFamily="monospace"
          minH="1.5em"
          opacity={0.7}
          wordBreak="break-all"
        >
          {latex ? `LaTeX: $${latex}$` : 'LaTeX가 자동으로 생성됩니다'}
        </Text>
      </VStack>
    </Flex>
  )
}
