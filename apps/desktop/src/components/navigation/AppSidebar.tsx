import { Flex, Text, VStack } from '@devup-ui/react'

import { SIDEBAR_ITEMS } from '@/constants/navigation'

export function AppSidebar() {
  return (
    <VStack
      as="aside"
      bg="$sidebarBackground"
      color="$sidebarText"
      h="100dvh"
      overflow="hidden"
      position="sticky"
      top="0"
    >
      <Flex
        alignItems="center"
        borderBottom="1px solid $sidebarBorder"
        gap="18px"
        minH="142px"
        px={['26px', null, null, '34px']}
      >
        <Text
          aria-hidden="true"
          color="$sidebarText"
          fontFamily="Segoe UI Symbol, sans-serif"
          fontSize="38px"
          lineHeight="1"
        >
          ⠿
        </Text>
        <VStack gap="2px">
          <Text as="h1" color="$sidebarText" typography="brandTitle">
            Braillify
          </Text>
          <Text color="$sidebarCaption" typography="sidebarBody">
            한글 점역 시스템
          </Text>
        </VStack>
      </Flex>

      <VStack aria-label="주요 화면" as="nav" gap="8px" p="18px">
        {SIDEBAR_ITEMS.map((item) => {
          const isActive = item.status === 'active'

          return (
            <Flex
              key={item.label}
              alignItems="center"
              aria-current={isActive ? 'page' : undefined}
              aria-disabled={!isActive || undefined}
              bg={isActive ? '$sidebarSelected' : 'transparent'}
              borderRadius="14px"
              color={isActive ? '$sidebarText' : '$sidebarDisabled'}
              gap="15px"
              minH="78px"
              px="18px"
            >
              <Text
                aria-hidden="true"
                color="currentColor"
                fontFamily="Segoe UI Symbol, sans-serif"
                fontSize="26px"
                lineHeight="1"
              >
                {item.symbol}
              </Text>
              <VStack gap="2px">
                <Text color="currentColor" typography="navigationTitle">
                  {item.label}
                </Text>
                <Text
                  color="currentColor"
                  opacity={0.62}
                  typography="sidebarBody"
                >
                  {item.description}
                </Text>
              </VStack>
            </Flex>
          )
        })}
      </VStack>

      <VStack
        borderTop="1px solid $sidebarBorder"
        gap="2px"
        mt="auto"
        px={['26px', null, null, '34px']}
        py="28px"
      >
        <Text color="$sidebarFooter" typography="sidebarCaption">
          braillify 오픈소스 엔진
        </Text>
        <Text color="$sidebarFooter" typography="sidebarCaption">
          2024 한국 점자 규정 기반
        </Text>
      </VStack>
    </VStack>
  )
}
