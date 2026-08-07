import { Box, Text } from '@devup-ui/react'
import type { ChangeEvent } from 'react'

type BrailleTextInputProps = {
  value: string
  onChange: (event: ChangeEvent<HTMLTextAreaElement>) => void
}

export function BrailleTextInput({ value, onChange }: BrailleTextInputProps) {
  return (
    <Box>
      <Text color="$textSubtle" fontSize="12px" fontWeight={600} mb="6px">
        직접 입력 / 붙여넣기
      </Text>
      <Box
        // 포커스 시 테두리 강조
        _focus={{ borderColor: '$primary' }}
        as="textarea"
        bg="$bg"
        border="1px solid"
        borderColor="$border"
        borderRadius="8px"
        color="$text"
        fontFamily="inherit"
        fontSize="12px"
        lineHeight={1.6}
        onChange={onChange}
        outline="none"
        placeholder="점자 유니코드를 붙여넣으세요  예: ⠈⠎⠐⠕"
        px="12px"
        py="10px"
        resize="none"
        rows={2}
        value={value}
        w="100%"
      />
      <Text color="$textSubtle" fontSize="11px" mt="4px">
        일반 키보드로는 점자 문자를 직접 타이핑하기 어려우므로 주로
        붙여넣기용입니다.
      </Text>
    </Box>
  )
}
