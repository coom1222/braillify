"use client";

import { useMemo, useState } from "react";
import { Box, Flex, Text } from "@devup-ui/react";
import { Toggle, Input } from "@devup-ui/components";
import {
  masksToString,
  mirrorMask,
  parseBrailleString,
  toggleDot,
  type DotNumber,
} from "../lib/braille";
import { EditableBrailleCell } from "../components/EditableBrailleCell";
import { copyText } from "../lib/clipboard";

export function EditorView() {
  const [cells, setCells] = useState<number[]>([0]);
  const [intaglio, setIntaglio] = useState(false);
  const [importInput, setImportInput] = useState("");
  const [importError, setImportError] = useState<string | null>(null);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">(
    "idle",
  );

  const previewMasks = useMemo(
    () => (intaglio ? cells.map(mirrorMask) : cells),
    [cells, intaglio],
  );
  const previewString = useMemo(
    () => masksToString(previewMasks),
    [previewMasks],
  );

  function handleToggleDot(cellIndex: number, dot: DotNumber) {
    setCells((prev) =>
      prev.map((m, i) => (i === cellIndex ? toggleDot(m, dot) : m)),
    );
  }
  function handleAddCell() {
    setCells((prev) => [...prev, 0]);
  }
  function handleReset() {
    setCells([0]);
  }
  function handleRemoveCell(cellIndex: number) {
    setCells((prev) => {
      if (prev.length <= 1) return [0];
      return prev.filter((_, i) => i !== cellIndex);
    });
  }
  function handleImport() {
    const trimmed = importInput.trim();
    if (!trimmed) {
      setImportError("점자 문자열을 붙여넣어주세요.");
      return;
    }
    const parsed = parseBrailleString(trimmed);
    if (parsed === null) {
      setImportError("U+2800 범위의 점자 문자만 사용할 수 있어요.");
      return;
    }
    setImportError(null);
    setCells(parsed.length > 0 ? parsed : [0]);
    setImportInput("");
  }
  async function handleCopy() {
    try {
      await copyText(previewString);
      setCopyState("copied");
      setTimeout(() => setCopyState("idle"), 1500);
    } catch {
      setCopyState("error");
      setTimeout(() => setCopyState("idle"), 1500);
    }
  }

  return (
    <Flex flexDirection="column" px="20px" pt="24px" pb="12px" gap="16px">
      <Box>
        <Text as="h1" fontSize="24px" fontWeight={700} m={0} mb="6px">
          점자 편집기
        </Text>
        <Text as="p" m={0} color="$textMuted" fontSize="14px">
          점 단위로 직접 점자를 조합하고 음각으로 양각 인쇄 레이아웃을
          확인하세요.
        </Text>
      </Box>

      {/* Preview card */}
      <Card>
        <Flex justifyContent="space-between" alignItems="center">
          <Text fontSize="14px" fontWeight={600}>
            미리보기
          </Text>
          <Flex alignItems="center" gap="12px">
            <Flex alignItems="center" gap="8px">
              <Text as="label" fontSize="13px" color="$textMuted">
                음각
              </Text>
              <Toggle
                variant="switch"
                value={intaglio}
                onChange={setIntaglio}
              />
            </Flex>
            <OutlineButton onClick={handleCopy}>
              {copyState === "copied"
                ? "복사됨"
                : copyState === "error"
                  ? "복사 실패"
                  : "복사"}
            </OutlineButton>
          </Flex>
        </Flex>
        <Box
          minH="64px"
          fontSize="32px"
          lineHeight={1.4}
          letterSpacing="4px"
          wordBreak="break-all"
          py="6px"
        >
          {previewString || " "}
        </Box>
      </Card>

      {/* Import card */}
      <Card>
        <Text fontSize="14px" fontWeight={600}>
          점자 가져오기
        </Text>
        <Input
          value={importInput}
          onChange={(e) => {
            setImportInput(e.target.value);
            setImportError(null);
          }}
          placeholder="점자 문자열을 붙여넣으세요 (U+2800 범위)"
          error={!!importError}
          errorMessage={importError ?? undefined}
          allowClear
          onClear={() => {
            setImportInput("");
            setImportError(null);
          }}
        />
        <Box
          as="button"
          type="button"
          bg="$primary"
          color="$primaryText"
          border={0}
          borderRadius="8px"
          py="12px"
          fontSize="14px"
          fontWeight={600}
          w="100%"
          onClick={handleImport}
        >
          가져오기
        </Box>
      </Card>

      {/* Cell editor card */}
      <Card>
        <Flex justifyContent="space-between" alignItems="center">
          <Text fontSize="14px" fontWeight={600}>
            점자 셀 편집 ({cells.length}셀)
          </Text>
          <Flex gap="8px">
            <OutlineButton onClick={handleAddCell}>+ 셀</OutlineButton>
            <Box
              as="button"
              type="button"
              bg="transparent"
              color="$danger"
              border="1px solid"
              borderColor="$danger"
              borderRadius="8px"
              px="14px"
              py="7px"
              fontSize="13px"
              fontWeight={600}
              onClick={handleReset}
            >
              초기화
            </Box>
          </Flex>
        </Flex>
        <Box
          display="grid"
          gridTemplateColumns="repeat(auto-fill, minmax(80px, 1fr))"
          gap="16px"
          py="4px"
        >
          {cells.map((mask, i) => (
            <EditableBrailleCell
              key={i}
              mask={mask}
              index={i}
              onToggleDot={(dot) => handleToggleDot(i, dot)}
              onRemove={() => handleRemoveCell(i)}
            />
          ))}
        </Box>
      </Card>

      {/* Dot number hint card */}
      <Card>
        <Text fontSize="14px" fontWeight={600}>
          점 번호
        </Text>
        <Text fontSize="13px" color="$textMuted">
          왼쪽: 1·2·3 / 오른쪽: 4·5·6
        </Text>
      </Card>
    </Flex>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return (
    <Flex
      flexDirection="column"
      bg="$surface"
      borderRadius="12px"
      border="1px solid"
      borderColor="$border"
      px="16px"
      py="14px"
      gap="10px"
    >
      {children}
    </Flex>
  );
}

function OutlineButton({
  children,
  onClick,
}: {
  children: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <Box
      as="button"
      type="button"
      bg="$surface"
      color="$text"
      border="1px solid"
      borderColor="$border"
      borderRadius="8px"
      px="14px"
      py="7px"
      fontSize="13px"
      fontWeight={500}
      onClick={onClick}
    >
      {children}
    </Box>
  );
}
