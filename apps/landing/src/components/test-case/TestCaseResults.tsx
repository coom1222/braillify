'use client'

import { Button, Flex, Text, VStack } from '@devup-ui/react'
import { useEffect, useState } from 'react'

import type { TestStatus, TestStatusPageInfo } from '@/types'

import { TestCaseList } from './list/TestCaseList'
import { TestCaseTable } from './table/TestCaseTable'
import { useTestCase } from './TestCaseProvider'

interface TestCaseResultsProps {
  pageInfo?: TestStatusPageInfo
  results: TestStatus[6]
  statusKey: string
  total: number
}

/**
 * Displays inline test results or lazily loads every page of a large result set.
 */
export function TestCaseResults({
  pageInfo,
  results,
  statusKey,
  total,
}: TestCaseResultsProps) {
  const { options } = useTestCase()
  const [page, setPage] = useState(1)
  const [pagedResults, setPagedResults] = useState<TestStatus[6]>([])
  const [isLoading, setIsLoading] = useState(Boolean(pageInfo))
  const [error, setError] = useState('')

  useEffect(() => {
    if (!pageInfo) return

    const abortController = new AbortController()
    const encodedStatusKey = statusKey
      .split('/')
      .map((segment) => encodeURIComponent(segment))
      .join('/')

    setIsLoading(true)
    setError('')
    fetch(`/test-status/${encodedStatusKey}/page-${page}.json`, {
      signal: abortController.signal,
    })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`)
        }
        return response.json() as Promise<TestStatus[6]>
      })
      .then((nextResults) => {
        setPagedResults(nextResults)
        setIsLoading(false)
      })
      .catch((fetchError: unknown) => {
        if (
          fetchError instanceof DOMException &&
          fetchError.name === 'AbortError'
        ) {
          return
        }
        setError('테스트 케이스를 불러오지 못했습니다.')
        setIsLoading(false)
      })

    return () => abortController.abort()
  }, [page, pageInfo, statusKey])

  function handleFirstPage() {
    setPage(1)
  }

  function handlePreviousPage() {
    setPage((currentPage) => Math.max(1, currentPage - 1))
  }

  function handleNextPage() {
    if (!pageInfo) return
    setPage((currentPage) => Math.min(pageInfo.pageCount, currentPage + 1))
  }

  function handleLastPage() {
    if (!pageInfo) return
    setPage(pageInfo.pageCount)
  }

  const visibleResults = pageInfo ? pagedResults : results
  const startIndex = pageInfo ? (page - 1) * pageInfo.pageSize : 0

  return (
    <VStack gap="20px">
      {pageInfo ? (
        <Flex
          alignItems="center"
          flexWrap="wrap"
          gap="8px"
          justifyContent="space-between"
        >
          <Text color="$caption" typography="body">
            {startIndex + 1}–{Math.min(startIndex + pageInfo.pageSize, total)} /{' '}
            {total.toLocaleString()}건
          </Text>
          <Flex alignItems="center" gap="8px">
            <Button
              _disabled={{ cursor: 'not-allowed', opacity: 0.4 }}
              border="solid 1px $primary"
              borderRadius="8px"
              color="$primary"
              cursor="pointer"
              disabled={page === 1}
              onClick={handleFirstPage}
              px="12px"
              py="6px"
            >
              처음
            </Button>
            <Button
              _disabled={{ cursor: 'not-allowed', opacity: 0.4 }}
              border="solid 1px $primary"
              borderRadius="8px"
              color="$primary"
              cursor="pointer"
              disabled={page === 1}
              onClick={handlePreviousPage}
              px="12px"
              py="6px"
            >
              이전
            </Button>
            <Text color="$text" typography="body">
              {page.toLocaleString()} / {pageInfo.pageCount.toLocaleString()}
            </Text>
            <Button
              _disabled={{ cursor: 'not-allowed', opacity: 0.4 }}
              border="solid 1px $primary"
              borderRadius="8px"
              color="$primary"
              cursor="pointer"
              disabled={page === pageInfo.pageCount}
              onClick={handleNextPage}
              px="12px"
              py="6px"
            >
              다음
            </Button>
            <Button
              _disabled={{ cursor: 'not-allowed', opacity: 0.4 }}
              border="solid 1px $primary"
              borderRadius="8px"
              color="$primary"
              cursor="pointer"
              disabled={page === pageInfo.pageCount}
              onClick={handleLastPage}
              px="12px"
              py="6px"
            >
              마지막
            </Button>
          </Flex>
        </Flex>
      ) : null}
      {isLoading ? (
        <Text color="$caption" typography="body">
          테스트 케이스를 불러오는 중입니다.
        </Text>
      ) : null}
      {error ? (
        <Text color="$error" typography="body">
          {error}
        </Text>
      ) : null}
      {!isLoading && !error && options.type === 'table' ? (
        <TestCaseTable results={visibleResults} startIndex={startIndex} />
      ) : null}
      {!isLoading && !error && options.type === 'list' ? (
        <TestCaseList results={visibleResults} />
      ) : null}
    </VStack>
  )
}
