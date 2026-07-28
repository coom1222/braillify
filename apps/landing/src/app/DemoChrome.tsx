import { Box, Flex, Image, Text } from '@devup-ui/react'

/** 데모 섹션 상단의 손가락 아이콘 + 안내 문구. 한글·수학 데모가 공유한다. */
export function DemoHeading({ children }: { children: string }) {
  return (
    <Flex
      alignItems="flex-start"
      gap={['10px', null, null, '20px']}
      justifyContent={['center', null, null, 'flex-start']}
    >
      <Box
        aria-hidden="true"
        bg="$text"
        flexShrink={0}
        h={['20px', null, null, '32px']}
        maskImage="url(/images/home/finger-point.svg)"
        maskPosition="center"
        maskRepeat="no-repeat"
        maskSize="contain"
        w={['17px', null, null, '28px']}
      />
      <Text color="$text" pos="relative" top="-2px" typography="mainTextSm">
        {children}
      </Text>
    </Flex>
  )
}

/** 입력 박스와 출력 박스 사이의 방향 화살표. 한글·수학 데모가 공유한다. */
export function DemoArrow() {
  return (
    <Flex aria-hidden="true">
      <Image
        alt=""
        display={['none', null, null, 'block']}
        mr="10px"
        role="presentation"
        src="/images/home/translate-arrow-circle.svg"
        w="16px"
      />
      <Image
        alt=""
        role="presentation"
        src="/images/home/translate-arrow.svg"
        transform={[null, null, null, 'rotate(-90deg)']}
        w={['16px', null, null, '24px']}
      />
    </Flex>
  )
}
