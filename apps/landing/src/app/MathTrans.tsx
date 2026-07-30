'use client'
import { VStack } from '@devup-ui/react'
import { useEffect, useState } from 'react'

import { DemoArrow, DemoHeading } from './DemoChrome'
import { MathTransInput } from './MathTransInput'
import { TransInput } from './TransInput'

export function MathTrans() {
  const [latex, setLatex] = useState('')
  const [translateToUnicode, setTranslateToUnicode] = useState<
    (input: string) => string
  >(() => () => '')
  useEffect(() => {
    import('braillify').then((mod) => {
      setTranslateToUnicode(() => (input: string) => {
        try {
          return mod.translateToUnicode(input)
        } catch (e) {
          console.error(e)
          return '점역할 수 없는 수식이 포함되어 있습니다.'
        }
      })
    })
  }, [])

  const braille = latex.trim() ? translateToUnicode(`$${latex}$`) : ''

  return (
    <VStack flex="1" gap={['16px', null, null, '30px']}>
      <DemoHeading>
        수식도 점자가 됩니다. 수식 키보드로 입력해 수학 점역을 체험해보세요!
      </DemoHeading>
      <VStack
        alignItems="center"
        flexDirection={[null, null, null, 'row']}
        gap={['12px', null, null, '30px']}
        h={['auto', null, null, '500px']}
      >
        <MathTransInput
          latex={latex}
          onLatexChange={setLatex}
          placeholder="수식을 입력하면 이곳에 수학 점자가 표시됩니다."
        />
        <DemoArrow />
        <TransInput
          blurPlaceholder={'예: 사분의 삼 → ⠼⠙⠌⠼⠉'}
          focusPlaceholder={'예: 사분의 삼 → ⠼⠙⠌⠼⠉'}
          readOnly
          value={braille}
        />
      </VStack>
    </VStack>
  )
}
