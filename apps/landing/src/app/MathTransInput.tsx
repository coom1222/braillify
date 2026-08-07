'use client'

import { css, Text, VStack } from '@devup-ui/react'
import type { MathfieldElement } from 'mathlive'
import { useEffect, useRef, useState } from 'react'

import { normalizeFracBraces } from './normalizeFracBraces'

declare global {
  // 커스텀 엘리먼트를 JSX 에 등록하려면 React.JSX 네임스페이스 병합이 유일한 방법이다.
  // oxlint-disable-next-line no-namespace
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
    <VStack
      bg="$containerBackground"
      borderRadius={['16px', null, null, '30px']}
      cursor="text"
      // 데스크톱 가로 배치에서 출력 박스와 폭을 반씩 나눈다. minW=0 은 math-field 의
      // 콘텐츠 최소 폭이 flex 아이템을 밀어 넓히는 것을 막는다.
      flex="1"
      gap="12px"
      h="100%"
      minH="25dvh"
      minW="0"
      onClick={() => fieldRef.current?.focus()}
      p={['16px', null, null, '40px']}
    >
      <VStack flex="1" gap="8px">
        {ready && (
          <math-field
            ref={fieldRef}
            className={css({
              background: 'transparent',
              border: 'none',
              display: 'block',
              fontSize: '28px',
              width: '100%',
            })}
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
      </VStack>
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
  )
}
