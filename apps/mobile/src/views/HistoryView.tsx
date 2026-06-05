"use client";

import { useEffect, useMemo, useState } from "react";
import { Box, Flex, Text } from "@devup-ui/react";
import { Input } from "@devup-ui/components";
import {
  listHistory,
  removeHistory,
  subscribeHistory,
  toggleFavorite,
  type HistoryItem,
} from "../lib/history";
import { copyText } from "../lib/clipboard";

type Tab = "recent" | "favorites";

export function HistoryView() {
  const [items, setItems] = useState<HistoryItem[]>(() => listHistory());
  const [tab, setTab] = useState<Tab>("recent");
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => subscribeHistory(() => setItems(listHistory())), []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return items
      .filter((it) => (tab === "favorites" ? it.favorite : true))
      .filter((it) => {
        if (!q) return true;
        return (
          it.source.toLowerCase().includes(q) ||
          it.braille.toLowerCase().includes(q)
        );
      });
  }, [items, tab, query]);

  async function handleCopy(item: HistoryItem) {
    try {
      await copyText(item.braille);
      setCopiedId(item.id);
      setTimeout(
        () => setCopiedId((cur) => (cur === item.id ? null : cur)),
        1200,
      );
    } catch {
      // ignore
    }
  }

  return (
    <Flex flexDirection="column" px="20px" pt="24px" pb="12px" gap="14px">
      <Box>
        <Text as="h1" fontSize="24px" fontWeight={700} m={0} mb="6px">
          점역 히스토리
        </Text>
        <Text as="p" m={0} color="$textMuted" fontSize="14px">
          최근 점역 작업 내역과 즐겨찾기를 관리합니다.
        </Text>
      </Box>

      {/* Tabs */}
      <Box
        display="grid"
        gridTemplateColumns="1fr 1fr"
        bg="$surface"
        borderRadius="12px"
        p="4px"
        border="1px solid"
        borderColor="$border"
      >
        {(["recent", "favorites"] as const).map((t) => {
          const active = tab === t;
          return (
            <Box
              key={t}
              as="button"
              type="button"
              role="tab"
              aria-selected={active}
              border={0}
              borderRadius="8px"
              py="10px"
              fontSize="13px"
              fontWeight={600}
              bg={active ? "$primary" : "transparent"}
              color={active ? "$primaryText" : "$textMuted"}
              onClick={() => setTab(t)}
            >
              {t === "recent" ? "🕐 최근 작업" : "⭐ 즐겨찾기"}
            </Box>
          );
        })}
      </Box>

      {/* Search */}
      <Input
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="검색..."
        allowClear
        onClear={() => setQuery("")}
      />

      {/* List */}
      <Flex flexDirection="column" gap="10px">
        {filtered.length === 0 && (
          <Box py="40px" textAlign="center" color="$textSubtle" fontSize="13px">
            {tab === "favorites"
              ? "즐겨찾기한 항목이 없어요."
              : query
                ? "검색 결과가 없어요."
                : "아직 점역 기록이 없어요."}
          </Box>
        )}
        {filtered.map((it) => {
          const isExpanded = expanded === it.id;
          return (
            <Box
              key={it.id}
              bg="$surface"
              border="1px solid"
              borderColor="$border"
              borderRadius="12px"
              px="16px"
              py="14px"
            >
              <Flex justifyContent="space-between" alignItems="center" gap="12px">
                <Flex
                  flex={1}
                  minW={0}
                  flexDirection="column"
                  gap="4px"
                >
                  <Box
                    fontSize="15px"
                    fontWeight={700}
                    overflow="hidden"
                    textOverflow="ellipsis"
                    whiteSpace="nowrap"
                  >
                    {it.source || "(빈 입력)"}
                  </Box>
                  <Box
                    fontSize="14px"
                    color="$textMuted"
                    letterSpacing="2px"
                    overflow={isExpanded ? "visible" : "hidden"}
                    textOverflow={isExpanded ? "clip" : "ellipsis"}
                    whiteSpace={isExpanded ? "normal" : "nowrap"}
                    wordBreak="break-all"
                  >
                    {it.braille}
                  </Box>
                </Flex>
                <Flex alignItems="center" gap="6px">
                  <Box
                    as="button"
                    type="button"
                    bg="transparent"
                    border={0}
                    fontSize="18px"
                    lineHeight={1}
                    px="2px"
                    aria-label={it.favorite ? "즐겨찾기 해제" : "즐겨찾기"}
                    aria-pressed={it.favorite}
                    onClick={() => toggleFavorite(it.id)}
                  >
                    {it.favorite ? "⭐" : "☆"}
                  </Box>
                  <Box
                    as="button"
                    type="button"
                    bg="$surface"
                    color="$text"
                    border="1px solid"
                    borderColor="$border"
                    borderRadius="8px"
                    px="12px"
                    py="6px"
                    fontSize="12px"
                    fontWeight={500}
                    onClick={() => handleCopy(it)}
                  >
                    {copiedId === it.id ? "복사됨" : "복사"}
                  </Box>
                  <Box
                    as="button"
                    type="button"
                    bg="transparent"
                    border={0}
                    color="$textSubtle"
                    fontSize="14px"
                    p="4px"
                    lineHeight={1}
                    aria-label="삭제"
                    onClick={() => removeHistory(it.id)}
                  >
                    ×
                  </Box>
                  <Box
                    as="button"
                    type="button"
                    bg="transparent"
                    border={0}
                    color="$textSubtle"
                    fontSize="14px"
                    p="4px"
                    lineHeight={1}
                    aria-label={isExpanded ? "접기" : "펼치기"}
                    onClick={() => setExpanded(isExpanded ? null : it.id)}
                  >
                    {isExpanded ? "▲" : "▼"}
                  </Box>
                </Flex>
              </Flex>
            </Box>
          );
        })}
      </Flex>
    </Flex>
  );
}
