import { Box, Flex, Grid, Text, VStack } from '@devup-ui/react'

export type TabKey = 'translator' | 'editor' | 'history'

const TABS: Array<{ key: TabKey; label: string; icon: string }> = [
  { key: 'translator', label: '점역기', icon: '⠿' },
  { key: 'editor', label: '편집기', icon: '⠶' },
  { key: 'history', label: '히스토리', icon: '⠒' },
]

interface BottomTabBarProps {
  active: TabKey
  onChange: (key: TabKey) => void
}

export function BottomTabBar({ active, onChange }: BottomTabBarProps) {
  return (
    <Grid
      as="nav"
      bg="$tabBar"
      bottom={0}
      gridTemplateColumns="repeat(3, 1fr)"
      pb="calc(env(safe-area-inset-bottom, 0px) + 8px)"
      position="sticky"
      pt="10px"
    >
      {TABS.map((tab) => {
        const isActive = tab.key === active
        return (
          <VStack
            key={tab.key}
            alignItems="center"
            aria-current={isActive ? 'page' : undefined}
            as="button"
            bg="transparent"
            border={0}
            color={isActive ? '#FFFFFF' : '#9A9A9A'}
            gap="4px"
            justifyContent="center"
            onClick={() => onChange(tab.key)}
            p="6px"
            type="button"
          >
            {/* 활성 dot 인디케이터 */}
            <Flex
              alignItems="center"
              height="28px"
              justifyContent="center"
              position="relative"
            >
              {isActive && (
                <Box
                  bg="#16887f"
                  borderRadius="50%"
                  h="4px"
                  left="50%"
                  position="absolute"
                  top="0"
                  transform="translateX(-50%) translateX(-1px)"
                  w="4px"
                />
              )}
              <Text fontSize="20px" lineHeight="1" mt="6px">
                {tab.icon}
              </Text>
            </Flex>
            <Text fontSize="12px" fontWeight={isActive ? 600 : 400}>
              {tab.label}
            </Text>
          </VStack>
        )
      })}
    </Grid>
  )
}
