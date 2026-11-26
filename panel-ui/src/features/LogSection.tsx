import { forwardRef, useState, useRef, useCallback, useEffect, useMemo } from "react";
import type { LogEntry } from "../types";
import { LOG_COLORS, HIGHLIGHT_RULES } from "../constants";
import { ToggleSwitch } from "../ui/ToggleSwitch";

const arrowBtnStyle: React.CSSProperties = {
  width: 22,
  height: 22,
  borderRadius: "50%",
  background: "#2a1a10",
  color: "#dcdcdc",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  cursor: "pointer",
  fontSize: 12,
  userSelect: "none",
  lineHeight: 1,
  flexShrink: 0,
  border: "none",
  fontFamily: "inherit",
  padding: 0,
};

export type LogFilter = "all" | "combat" | "action" | "highlight";

const logButtonClassName = "cursor-pointer rounded-sm border-0 bg-panel-border px-2.5 py-1.5 text-xs text-panel-text hover:bg-[#4A2A15] hover:text-panel-gold";

export const LogSection = forwardRef<
  HTMLDivElement,
  {
    entries: LogEntry[];
    totalRounds: number;
    currentRound: number;
    autoScroll: boolean;
    logFilter: LogFilter;
    onAutoScrollChange: (v: boolean) => void;
    onFilterChange: (v: LogFilter) => void;
    onPageChange: (page: number) => void;
    onClear: () => void;
    onExportCurrent: () => void;
    onExportAll: () => void;
  }
>(function LogSection(
  {
    entries,
    totalRounds,
    currentRound,
    autoScroll,
    logFilter,
    onAutoScrollChange,
    onFilterChange,
    onPageChange,
    onClear,
    onExportCurrent,
    onExportAll,
  },
  logEndRef
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [pageInput, setPageInput] = useState(String(currentRound + 1));
  const [monsterFilter, setMonsterFilter] = useState(-1);
  const pageInputRef = useRef<HTMLInputElement>(null);
  const prevRoundRef = useRef(currentRound);
  const lastScrollTopRef = useRef(0);
  const touchYRef = useRef<number | null>(null);
  const draggingScrollbarRef = useRef(false);

  const scrollToTop = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      const el = containerRef.current;
      if (!el) return;

      el.scrollTo({ top: 0, behavior });
    },
    []
  );

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "smooth") => {
      const el = containerRef.current;
      if (!el) return;

      el.scrollTo({ top: el.scrollHeight, behavior });
    },
    []
  );

  useEffect(() => {
    const stopScrollbarDrag = () => {
      draggingScrollbarRef.current = false;
    };

    window.addEventListener("pointerup", stopScrollbarDrag);
    window.addEventListener("pointercancel", stopScrollbarDrag);

    return () => {
      window.removeEventListener("pointerup", stopScrollbarDrag);
      window.removeEventListener("pointercancel", stopScrollbarDrag);
    };
  }, []);

  // sync pageInput when currentRound changes externally
  useEffect(() => {
    setPageInput(String(currentRound + 1));
  }, [currentRound]);

  // 切换页只处理当前页内部滚动位置，不再参与“是否跳最新页”的判断。
  useEffect(() => {
    if (prevRoundRef.current === currentRound) return;

    prevRoundRef.current = currentRound;

    if (autoScroll) {
      scrollToBottom("auto");
    } else {
      scrollToTop("auto");
    }
  }, [currentRound, autoScroll, scrollToBottom, scrollToTop]);

  // 自动滚动只针对当前页：当前页日志条数变化时才滚到底部。
  useEffect(() => {
    if (!autoScroll || entries.length === 0) return;

    scrollToBottom("smooth");
  }, [entries.length, autoScroll, scrollToBottom]);

  // 换页时重置怪物筛选
  useEffect(() => {
    setMonsterFilter(-1);
  }, [currentRound]);

  const disableAutoScroll = useCallback(() => {
    if (autoScroll) {
      onAutoScrollChange(false);
    }
  }, [autoScroll, onAutoScrollChange]);

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (event.deltaY < 0 && event.currentTarget.scrollTop > 0) {
        disableAutoScroll();
      }
    },
    [disableAutoScroll]
  );

  const handleTouchStart = useCallback((event: React.TouchEvent<HTMLDivElement>) => {
    touchYRef.current = event.touches[0]?.clientY ?? null;
  }, []);

  const handleTouchMove = useCallback(
    (event: React.TouchEvent<HTMLDivElement>) => {
      const currentY = event.touches[0]?.clientY;
      const previousY = touchYRef.current;

      if (currentY == null) return;

      // 手指向下移动时，日志内容向下移动，即用户正在查看更早的日志。
      if (previousY != null && currentY > previousY && event.currentTarget.scrollTop > 0) {
        disableAutoScroll();
      }

      touchYRef.current = currentY;
    },
    [disableAutoScroll]
  );

  const handleTouchEnd = useCallback(() => {
    touchYRef.current = null;
  }, []);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      const scrollsUp =
        event.key === "ArrowUp" ||
        event.key === "PageUp" ||
        event.key === "Home" ||
        (event.key === " " && event.shiftKey);

      if (scrollsUp && event.currentTarget.scrollTop > 0) {
        disableAutoScroll();
      }
    },
    [disableAutoScroll]
  );

  const handlePointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const el = event.currentTarget;
    const rect = el.getBoundingClientRect();
    const scrollbarWidth = Math.max(6, el.offsetWidth - el.clientWidth);

    // 只把滚动条区域内的按下视为拖动，避免普通的日志文本选择影响自动滚动。
    draggingScrollbarRef.current =
      event.pointerType === "mouse" && event.clientX >= rect.right - scrollbarWidth;
    lastScrollTopRef.current = el.scrollTop;
  }, []);

  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;

    const previousScrollTop = lastScrollTopRef.current;
    const currentScrollTop = el.scrollTop;
    lastScrollTopRef.current = currentScrollTop;

    // scroll 事件本身无法区分换页、代码滚动和用户操作；仅在拖动滚动条且方向向上时关闭。
    if (draggingScrollbarRef.current && currentScrollTop < previousScrollTop) {
      disableAutoScroll();
    }
  }, [disableAutoScroll]);

  const handlePageSubmit = useCallback(() => {
    const p = parseInt(pageInput, 10);

    if (!Number.isNaN(p) && p >= 1 && p <= totalRounds) {
      onPageChange(p - 1);
    } else {
      setPageInput(String(currentRound + 1));
    }
  }, [pageInput, totalRounds, currentRound, onPageChange]);

  const goPrevPage = useCallback(() => {
    if (currentRound > 0) {
      onPageChange(currentRound - 1);
    }
  }, [currentRound, onPageChange]);

  const goNextPage = useCallback(() => {
    if (currentRound < totalRounds - 1) {
      onPageChange(currentRound + 1);
    }
  }, [currentRound, totalRounds, onPageChange]);

  const filteredEntries = entries.filter((e) => {
    if (logFilter === "all") return true;
    if (logFilter === "combat") return e.level === "Combat";
    if (logFilter === "action") return e.action_id != null;
    if (logFilter === "highlight") return HIGHLIGHT_RULES.some((r) => r.match(e));
    return true;
  }).filter((e) => {
    if (monsterFilter === -1) return true;
    return e.monster_id === monsterFilter;
  });

  const filterOptions = [
    { value: "all" as const, label: "全部显示" },
    { value: "combat" as const, label: "仅伤害记录" },
    { value: "action" as const, label: "仅出招记录" },
    { value: "highlight" as const, label: "仅关键信息" },
  ];

  // 从当前页条目中提取去重的怪物列表
  const monsterOptions = useMemo(() => {
    const map = new Map<number, string>();
    for (const e of entries) {
      if (e.monster_id != null && e.monster_name) {
        map.set(e.monster_id, e.monster_name);
      }
    }
    if (map.size <= 1) return [];
    return [
      { id: -1, name: "全部怪物" },
      ...Array.from(map, ([id, name]) => ({ id, name })),
    ];
  }, [entries]);

  return (
    <section aria-label="狩猎日志" className="flex flex-col p-4">
      <div className="mb-2.5 flex flex-wrap items-center gap-1.5">
        <h2 className="m-0 text-base font-semibold text-panel-text">狩猎日志</h2>

        <span style={{ color: "#dcdcdc", fontSize: 12, marginLeft: 4 }}>
          {filteredEntries.length} 条 / {totalRounds} 轮
        </span>

        {/* 分页导航（跟随标题，totalRounds=0 时隐藏） */}
        {totalRounds > 0 && (
          <div style={{ display: "flex", alignItems: "center", gap: 3, marginLeft: 8 }}>
            <button
              style={{
                ...arrowBtnStyle,
                opacity: currentRound <= 0 ? 0.45 : 1,
                cursor: currentRound <= 0 ? "not-allowed" : "pointer",
              }}
              disabled={currentRound <= 0}
              onClick={goPrevPage}
              title="上一页"
            >
              ◀
            </button>

            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 2,
                color: "#dcdcdc",
                fontSize: 14,
                userSelect: "none",
              }}
            >
              <input
                ref={pageInputRef}
                aria-label="日志页码"
                inputMode="numeric"
                value={pageInput}
                onChange={(e) => setPageInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handlePageSubmit();
                }}
                onBlur={handlePageSubmit}
                style={{
                  width: 32,
                  textAlign: "center",
                  background: "#1a0f08",
                  border: "1px solid #443322",
                  borderRadius: 3,
                  color: "#dcdcdc",
                  fontSize: 14,
                  padding: "2px 0",
                  outline: "none",
                }}
              />

              <span style={{ color: "#dcdcdc" }}>/ {totalRounds}</span>
            </div>

            <button
              style={{
                ...arrowBtnStyle,
                opacity: currentRound >= totalRounds - 1 ? 0.45 : 1,
                cursor: currentRound >= totalRounds - 1 ? "not-allowed" : "pointer",
              }}
              disabled={currentRound >= totalRounds - 1}
              onClick={goNextPage}
              title="下一页"
            >
              ▶
            </button>
          </div>
        )}

        {/* 筛选按钮 */}
        <div className="flex flex-wrap gap-1.5">
          {filterOptions.map((opt) => (
            <button
              key={opt.value}
              type="button"
              aria-pressed={logFilter === opt.value}
              onClick={() => onFilterChange(opt.value)}
              className={`${logButtonClassName} aria-pressed:bg-[#5C3A1E] aria-pressed:text-panel-gold`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* 怪物筛选下拉 */}
        {monsterOptions.length > 0 && (
          <select
            aria-label="筛选怪物"
            value={monsterFilter}
            onChange={(e) => setMonsterFilter(Number(e.target.value))}
            style={{
              background: monsterFilter === -1 ? "#331e12" : "#5c3a1e",
              color: monsterFilter === -1 ? "#dcdcdc" : "#ffddaa",
              border: "1px solid #443322",
              borderRadius: 3,
              padding: "3px 6px",
              fontSize: 13,
              fontFamily: "inherit",
              outline: "none",
              cursor: "pointer",
              height: 26,
              alignSelf: "center",
            }}
          >
            {monsterOptions.map((opt) => (
              <option key={opt.id} value={opt.id}>
                {opt.name}
              </option>
            ))}
          </select>
        )}

        {/* 自动滚动只控制当前页内部滚动 */}
        <div style={{ marginLeft: "auto" }}>
          <ToggleSwitch checked={autoScroll} onChange={onAutoScrollChange} label="自动滚动" />
        </div>
      </div>

      {/* ── 日志容器 ── */}
      <div
        ref={containerRef}
        className="log-container h-90 flex-none overflow-y-auto rounded-sm border border-solid border-panel-border bg-black/25 p-2 text-xs leading-relaxed tabular-nums md:text-sm"
        tabIndex={0}
        onWheel={handleWheel}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
        onTouchCancel={handleTouchEnd}
        onKeyDown={handleKeyDown}
        onPointerDown={handlePointerDown}
        onScroll={handleScroll}
      >
        {filteredEntries.length === 0 ? (
          <div
            style={{
              color: "#8c8c8c",
              fontStyle: "italic",
              marginTop: 20,
              textAlign: "center",
            }}
          >
            {entries.length === 0
              ? totalRounds <= 0
                ? "暂无狩猎记录"
                : "等待游戏数据..."
              : "当前筛选条件下无匹配记录"}
          </div>
        ) : (
          filteredEntries.map((entry, i) => {
            const highlight = HIGHLIGHT_RULES.find((r) => r.match(entry));
            const baseColor = LOG_COLORS[entry.level] ?? "#dcdcdc";

            return (
              <div
                key={i}
                style={{
                  backgroundColor: highlight?.style.backgroundColor ?? "transparent",
                  color: highlight?.style.color ?? baseColor,
                  paddingLeft: 4,
                  borderRadius: 2,
                }}
              >
                <span style={{ color: "#8c8c8c" }}>{entry.timestamp}</span>{" "}
                <span>{entry.message}</span>
              </div>
            );
          })
        )}

        <div ref={logEndRef} />
      </div>

      <div className="mt-2.5 flex flex-wrap items-center gap-2">
        <span className="mr-auto text-xs text-panel-muted">每场狩猎的日志独立成页</span>
        <button type="button" onClick={onClear} className={logButtonClassName}>清除日志</button>
        <button type="button" onClick={onExportAll} className={logButtonClassName}>导出全部</button>
        <button type="button" onClick={onExportCurrent} className={logButtonClassName}>导出本页</button>
      </div>
    </section>
  );
});
