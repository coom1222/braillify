import { Button, Flex, Input, Text, VStack } from '@devup-ui/react'

type TranslationInputCardProps = {
  errorMessage: string | null
  input: string
  isTranslating: boolean
  onChange: (value: string) => void
  onSubmit: () => void
}

export function TranslationInputCard({
  errorMessage,
  input,
  isTranslating,
  onChange,
  onSubmit,
}: TranslationInputCardProps) {
  const characterCount = Array.from(input).length

  return (
    <VStack
      as="section"
      bg="$containerBackground"
      border="1px solid $border"
      borderRadius="20px"
      boxShadow="0 1px 3px rgba(34, 34, 34, 0.04)"
      overflow="hidden"
      w="100%"
    >
      <Flex
        alignItems="center"
        borderBottom="1px solid $border"
        justifyContent="space-between"
        minH="66px"
        px="24px"
      >
        <Text as="h2" typography="inputTitle">
          입력 텍스트
        </Text>
        <Text aria-live="polite" color="$caption" typography="body">
          {characterCount}자
        </Text>
      </Flex>

      <Input
        aria-describedby="translation-help"
        aria-invalid={errorMessage ? true : undefined}
        aria-label="점역할 텍스트"
        as="textarea"
        bg="transparent"
        border="none"
        color="$text"
        minH="224px"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.ctrlKey && event.key === 'Enter') {
            event.preventDefault()
            onSubmit()
          }
        }}
        p="26px"
        placeholder="점역할 텍스트를 입력하세요..."
        resize="none"
        value={input}
        w="100%"
      />

      <Flex
        alignItems="center"
        borderTop="1px solid $border"
        gap="18px"
        justifyContent="flex-end"
        minH="88px"
        px="24px"
      >
        <Text
          color={errorMessage ? '$error' : '$caption'}
          id="translation-help"
          role={errorMessage ? 'alert' : undefined}
          typography="body"
        >
          {errorMessage || 'Ctrl + Enter로 변환'}
        </Text>
        <Button
          bg={
            isTranslating || input.trim().length === 0
              ? '$disabledBackground'
              : '$primary'
          }
          border="none"
          borderRadius="13px"
          color={
            isTranslating || input.trim().length === 0
              ? '$disabledText'
              : '$base'
          }
          cursor={
            isTranslating
              ? 'wait'
              : input.trim().length === 0
                ? 'default'
                : 'pointer'
          }
          disabled={isTranslating || input.trim().length === 0}
          minW="132px"
          onClick={onSubmit}
          px="22px"
          py="15px"
          typography="button"
        >
          {isTranslating ? '점역 중…' : '점역하기'}
        </Button>
      </Flex>
    </VStack>
  )
}
