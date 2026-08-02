import { Button, Text, VStack } from '@devup-ui/react'

export type CopyState = 'idle' | 'copied' | 'error'

const COPY_STATUS = {
  copied: '결과를 클립보드에 복사했습니다.',
  error: '결과를 복사하지 못했습니다. 다시 시도해 주세요.',
  idle: '',
} as const satisfies Record<CopyState, string>

type TranslationOutputProps = {
  copyState: CopyState
  onCopy: () => void
  result: string
}

export function TranslationOutput({
  copyState,
  onCopy,
  result,
}: TranslationOutputProps) {
  if (!result) {
    return (
      <VStack
        alignItems="center"
        aria-label="점역 결과"
        aria-live="polite"
        as="output"
        gap="20px"
        justifyContent="center"
        minH="430px"
        py="44px"
        w="100%"
      >
        <Text
          aria-hidden="true"
          color="$emptyBraille"
          fontFamily="Segoe UI Symbol, sans-serif"
          fontSize={['36px', null, '42px']}
          letterSpacing="0.3em"
          lineHeight="1"
        >
          ⠃⠗⠁⠊⠇⠇⠊⠋⠽
        </Text>
        <Text color="$caption" typography="body">
          텍스트를 입력하고 점역을 시작해보세요
        </Text>
      </VStack>
    )
  }

  return (
    <VStack
      alignItems="center"
      aria-label="점역 결과"
      aria-live="polite"
      as="section"
      gap="24px"
      justifyContent="center"
      minH="430px"
      py="44px"
      w="100%"
    >
      <Text
        aria-label="점역 결과 텍스트"
        as="output"
        color="$text"
        maxW="920px"
        tabIndex={0}
        textAlign="center"
        typography="braille"
        userSelect="text"
        whiteSpace="pre-wrap"
        wordBreak="break-word"
      >
        {result}
      </Text>
      <Button
        aria-keyshortcuts="Control+Shift+C"
        bg="$containerBackground"
        border="1px solid $border"
        borderRadius="12px"
        color="$text"
        cursor="pointer"
        onClick={onCopy}
        px="18px"
        py="11px"
        typography="button"
      >
        결과 복사
      </Button>
      <Text
        color={copyState === 'error' ? '$error' : '$caption'}
        minH="22px"
        role={copyState === 'error' ? 'alert' : 'status'}
        typography="body"
      >
        {COPY_STATUS[copyState]}
      </Text>
    </VStack>
  )
}
