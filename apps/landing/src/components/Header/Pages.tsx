'use client'

import { Box, Center, css, Flex, Text } from '@devup-ui/react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'

export default function Pages({ isIntersecting }: { isIntersecting: boolean }) {
  const pathname = usePathname()

  return (
    <Flex
      alignItems="center"
      color="$text"
      display={['none', null, null, 'flex']}
      flexGrow={isIntersecting ? 1 : 1}
      justifyContent="flex-end"
      transform={isIntersecting ? 'translateX(0)' : 'translateX(-50%)'}
      transition="all 0.3s ease"
    >
      <Flex
        gap={isIntersecting ? '0px' : '40px'}
        transform={isIntersecting ? 'translateX(0)' : 'translateX(50%)'}
        transition="all 0.3s ease"
      >
        <Link
          aria-label="Overview page link"
          className={css({
            color: '$text',
          })}
          href="/docs/overview"
        >
          <Flex alignItems="center" p="40px">
            <Center data-group gap="10px">
              <Box
                _groupActive={{
                  bg: '$text',
                }}
                _groupHover={{
                  bg: '$text',
                }}
                aspectRatio="1"
                bg={pathname.startsWith('/docs') ? '$text' : 'transparent'}
                border="1px solid $text"
                borderRadius="50%"
                h="12px"
                transition="all 0.2s ease"
              />
              <Text
                typography={
                  pathname.startsWith('/docs') ? 'gnbMenuBold' : 'gnbMenu'
                }
              >
                문서
              </Text>
            </Center>
          </Flex>
        </Link>
        <Link
          aria-label="Test case page link"
          className={css({
            color: '$text',
          })}
          href="/test-case"
        >
          <Flex alignItems="center" p="40px">
            <Center data-group gap="10px">
              <Box
                _groupActive={{
                  bg: '$text',
                }}
                _groupHover={{
                  bg: '$text',
                }}
                aspectRatio="1"
                bg={pathname.startsWith('/test-case') ? '$text' : 'transparent'}
                border="1px solid $text"
                borderRadius="50%"
                h="12px"
                transition="all 0.2s ease"
              />
              <Text
                typography={
                  pathname.startsWith('/test-case') ? 'gnbMenuBold' : 'gnbMenu'
                }
              >
                테스트 케이스
              </Text>
            </Center>
          </Flex>
        </Link>
        <Link
          aria-label="Team page link"
          className={css({
            color: '$text',
          })}
          href="/team"
        >
          <Flex alignItems="center" p="40px">
            <Center data-group gap="10px">
              <Box
                _groupActive={{
                  bg: '$text',
                }}
                _groupHover={{
                  bg: '$text',
                }}
                aspectRatio="1"
                bg={pathname.startsWith('/team') ? '$text' : 'transparent'}
                border="1px solid $text"
                borderRadius="50%"
                h="12px"
                transition="all 0.2s ease"
              />
              <Text
                typography={
                  pathname.startsWith('/team') ? 'gnbMenuBold' : 'gnbMenu'
                }
              >
                팀 소개
              </Text>
            </Center>
          </Flex>
        </Link>
      </Flex>
    </Flex>
  )
}
