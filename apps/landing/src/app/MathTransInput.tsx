'use client'

import { Flex, Text, VStack } from '@devup-ui/react'
import type { MathfieldElement } from 'mathlive'
import { useEffect, useRef, useState } from 'react'

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

/**
 * MathLive는 한 글자 인자를 중괄호 없이 직렬화한다 (예: \frac34, \frac{3}4).
 * braillify의 LaTeX 파서는 \frac{분자}{분모} 형태만 분수로 인식하므로
 * \frac의 두 인자를 항상 중괄호로 감싼 정규형으로 변환한다.
 */
export function normalizeFracBraces(latex: string): string {
  let out = ''
  let i = 0
  while (i < latex.length) {
    if (latex.startsWith('\\frac', i) && !/[a-zA-Z]/.test(latex[i + 5] ?? '')) {
      const [num, j] = readArg(latex, i + 5)
      const [den, k] = readArg(latex, j)
      out += `\\frac${num}${den}`
      i = k
      continue
    }
    out += latex[i]
    i += 1
  }
  return out
}

/** i 위치부터 LaTeX 인자 하나를 읽어 중괄호로 감싼 형태와 다음 인덱스를 돌려준다. */
function readArg(latex: string, i: number): [string, number] {
  while (latex[i] === ' ') i += 1
  if (latex[i] === '{') {
    let depth = 0
    let j = i
    do {
      if (latex[j] === '{') depth += 1
      else if (latex[j] === '}') depth -= 1
      j += 1
    } while (j < latex.length && depth > 0)
    // depth === 0 이면 짝이 맞는 '}' 를 j-1 에서 소비한 것이고,
    // depth > 0 이면 닫는 중괄호 없이 끝난 것이라 내용을 j 까지 살린다.
    const end = depth === 0 ? j - 1 : j
    return [`{${normalizeFracBraces(latex.slice(i + 1, end))}}`, j]
  }
  if (latex[i] === '\\') {
    let j = i + 1
    while (j < latex.length && /[a-zA-Z]/.test(latex[j] ?? '')) j += 1
    // 제어기호(\, \! 등)는 백슬래시 뒤에 글자가 없으므로 기호 한 글자를 포함시킨다.
    if (j === i + 1 && j < latex.length) j += 1
    return [`{${latex.slice(i, j)}}`, j]
  }
  return [`{${latex[i] ?? ''}}`, i + 1]
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
        gap="12px"
        h="100%"
        minH="25dvh"
        onClick={() => fieldRef.current?.focus()}
        p={['16px', null, null, '40px']}
        w="100%"
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
              style={{
                background: 'transparent',
                border: 'none',
                display: 'block',
                fontSize: '28px',
                width: '100%',
              }}
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
