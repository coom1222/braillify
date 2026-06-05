"use client";

import { useEffect, useRef, useState } from "react";
import { Box, Flex, Text } from "@devup-ui/react";
import { masksToString, toggleDot, type DotNumber } from "../lib/braille";
import { EditableBrailleCell } from "./EditableBrailleCell";

interface BrailleInputProps {
  cells: number[];
  onChange: (cells: number[]) => void;
}

// 브라유 유니코드 범위: U+2800 ~ U+283F
const BRAILLE_START = 0x2800;
const BRAILLE_END = 0x283f;

function parseToCells(raw: string): number[] {
  const result: number[] = [];
  for (const ch of raw) {
    const code = ch.codePointAt(0) ?? 0;
    if (code >= BRAILLE_START && code <= BRAILLE_END) {
      result.push(code - BRAILLE_START);
    } else if (ch === " " || ch === "\n") {
      result.push(0); // 일반 공백·줄바꿈 → ⠀(U+2800) 점자 공백으로 보존
    }
    // 그 외 문자는 무시
  }
  return result;
}

export function BrailleInput({ cells, onChange }: BrailleInputProps) {
  // 각 셀에 안정적인 고유 key를 부여하기 위한 카운터 + id 배열
  // (배열 인덱스를 key로 쓰면 삭제 시 React가 잘못된 컴포넌트를 재사용할 수 있음)
  const counter = useRef(0);
  const nextId = () => counter.current++;
  const [cellIds, setCellIds] = useState<number[]>([]);

  // 외부에서 cells가 초기화될 때(방향 전환 등) ids도 동기화
  useEffect(() => {
    if (cells.length === 0) setCellIds([]);
  }, [cells.length]);

  // ── 텍스트 직접 입력 ──────────────────────────────────────────
  function handleTextChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const newCells = parseToCells(e.target.value);
    // 텍스트 붙여넣기로 전체 교체 → 모든 셀에 새 id 발급
    setCellIds(newCells.map(() => nextId()));
    onChange(newCells);
  }

  // ── 도트 키보드 ───────────────────────────────────────────────
  function handleToggleDot(cellIndex: number, dot: DotNumber) {
    // 점 토글은 셀 구조가 바뀌지 않으므로 id 변경 불필요
    onChange(cells.map((m, i) => (i === cellIndex ? toggleDot(m, dot) : m)));
  }

  function handleAddCell() {
    setCellIds((ids) => [...ids, nextId()]);
    onChange([...cells, 0]);
  }

  function handleAddSpace() {
    setCellIds((ids) => [...ids, nextId()]);
    onChange([...cells, 0]); // mask 0 → ⠀ (점자 공백)
  }

  function handleBackspace() {
    if (cells.length > 0) {
      setCellIds((ids) => ids.slice(0, -1));
      onChange(cells.slice(0, -1));
    }
  }

  function handleRemoveCell(i: number) {
    setCellIds((ids) => ids.filter((_, idx) => idx !== i));
    onChange(cells.filter((_, idx) => idx !== i));
  }

  function handleClear() {
    setCellIds([]);
    onChange([]);
  }

  const hasContent = cells.length > 0;
  const unicodeValue = masksToString(cells);

  return (
    <Box display="flex" flexDirection="column" gap="14px">
      {/* ── 섹션 1: 직접 입력 ─────────────────────────────── */}
      <Box>
        <Text fontSize="12px" fontWeight={600} color="$textSubtle" mb="6px">
          직접 입력 / 붙여넣기
        </Text>
        <Box
          as="textarea"
          value={unicodeValue}
          onChange={handleTextChange}
          rows={2}
          placeholder={"점자 유니코드를 붙여넣으세요  예: ⠈⠎⠐⠕"}
          w="100%"
          bg="$background"
          color="$text"
          border="1px solid"
          borderColor="$border"
          borderRadius="8px"
          px="12px"
          py="10px"
          fontSize="12px"
          lineHeight={1.6}
          resize="none"
          outline="none"
          fontFamily="inherit"
          // 포커스 시 테두리 강조
          _focus={{ borderColor: "$primary" }}
        />
        <Text fontSize="11px" color="$textSubtle" mt="4px">
          일반 키보드로는 점자 문자를 직접 타이핑하기 어려우므로 주로
          붙여넣기용입니다.
        </Text>
      </Box>

      {/* 구분선 */}
      <Flex alignItems="center" gap="10px">
        <Box flex={1} h="1px" bg="$border" />
        <Text fontSize="11px" color="$textSubtle" flexShrink={0}>
          또는 도트 키보드로 조합
        </Text>
        <Box flex={1} h="1px" bg="$border" />
      </Flex>

      {/* ── 섹션 2: 도트 키보드 ───────────────────────────── */}
      <Box>
        <Text fontSize="12px" fontWeight={600} color="$textSubtle" mb="8px">
          도트 키보드
        </Text>

        {/* 셀 편집 그리드 */}
        {hasContent ? (
          <Box
            display="grid"
            gridTemplateColumns="repeat(auto-fill, minmax(80px, 1fr))"
            gap="12px"
            mb="12px"
            maxH="220px"
            overflowY="auto"
          >
            {cells.map((mask, i) => (
              <EditableBrailleCell
                key={cellIds[i] ?? i}
                mask={mask}
                index={i}
                onToggleDot={(dot) => handleToggleDot(i, dot)}
                onRemove={hasContent ? () => handleRemoveCell(i) : undefined}
              />
            ))}
          </Box>
        ) : (
          <Flex
            alignItems="center"
            justifyContent="center"
            border="1px dashed"
            borderColor="$border"
            borderRadius="8px"
            py="16px"
            mb="12px"
            color="$textSubtle"
            fontSize="12px"
          >
            왼쪽: 점 1·2·3 &nbsp;/&nbsp; 오른쪽: 점 4·5·6
          </Flex>
        )}

        {/* 액션 버튼 */}
        <Flex gap="8px" flexWrap="wrap">
          <Box
            as="button"
            type="button"
            border="1px solid"
            borderColor="$border"
            bg="$surface"
            color="$text"
            borderRadius="8px"
            px="14px"
            py="9px"
            fontSize="14px"
            fontWeight={500}
            disabled={!hasContent}
            opacity={!hasContent ? 0.4 : 1}
            cursor={!hasContent ? "not-allowed" : "pointer"}
            onClick={handleBackspace}
            aria-label="마지막 셀 지우기"
          >
            ⌫
          </Box>

          <Box
            as="button"
            type="button"
            border="1px solid"
            borderColor="$border"
            bg="$surface"
            color="$text"
            borderRadius="8px"
            px="14px"
            py="9px"
            fontSize="13px"
            fontWeight={500}
            cursor="pointer"
            onClick={handleAddSpace}
          >
            공백 ⠀
          </Box>

          <Box
            as="button"
            type="button"
            border={0}
            bg="$primary"
            color="$primaryText"
            borderRadius="8px"
            px="16px"
            py="9px"
            fontSize="13px"
            fontWeight={600}
            cursor="pointer"
            onClick={handleAddCell}
          >
            + 셀
          </Box>

          {hasContent && (
            <Box
              as="button"
              type="button"
              bg="transparent"
              color="$danger"
              border="1px solid"
              borderColor="$danger"
              borderRadius="8px"
              px="14px"
              py="9px"
              fontSize="13px"
              fontWeight={500}
              cursor="pointer"
              onClick={handleClear}
            >
              전체 삭제
            </Box>
          )}
        </Flex>
      </Box>
    </Box>
  );
}
