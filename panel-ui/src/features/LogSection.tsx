import { forwardRef, useState, useRef, useCallback, useEffect } from "react";
import type { LogEntry } from "../types";
import { LOG_COLORS, HIGHLIGHT_RULES, btnStyle } from "../constants";

const PROGRAMMATIC_SCROLL_SUPPRESS_MS = 700;

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
  fontSize: 11,
  userSelect: "none",
  lineHeight: 1,
  flexShrink: 0,
  border: "none",
  fontFamily: "inherit",
  padding: 0,
};

function ToggleSwitch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        cursor: "pointer",
        userSelect: "none",
      }}
      onClick={() => onChange(!checked)}
    >
      <div
        style={{
          width: 38,
          height: 20,
          borderRadius: 10,
          flexShrink: 0,
          background: checked ? "linear-gradient(#BFA76B, #6C552D)" : "#444444",
          position: "relative",
          border: "1px solid #666",
          transition: "background 0.1s",
        }}
      >
        <div
          style={{
            width: 16,
            height: 16,
            borderRadius: "50%",
            background: "#fff",
            position: "absolute",
            top: 1,
            left: checked ? 20 : 2,
            transition: "left 0.1s",
          }}
        />
      </div>

      <span style={{ color: "#b0b0b0", fontSize: 13 }}>{label}</span>
    </div>
  );
}

export const LogSection = forwardRef<
  HTMLDivElement,
  {
    entries: LogEntry[];
    totalRounds: number;
    currentRound: number;
    autoScroll: boolean;
    onAutoScrollChange: (v: boolean) => void;
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
    onAutoScrollChange,
    onPageChange,
    onClear,
    onExportCurrent,
    onExportAll,
  },
  logEndRef
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [pageInput, setPageInput] = useState(String(currentRound + 1));
  const pageInputRef = useRef<HTMLInputElement>(null);
  const prevRoundRef = useRef(currentRound);

  // 用于区分代码触发滚动和用户触发滚动。
  const suppressScrollRef = useRef(false);
  const suppressScrollTimerRef = useRef<number | null>(null);

  const clearProgrammaticScrollMark = useCallback(() => {
    suppressScrollRef.current = false;

    if (suppressScrollTimerRef.current != null) {
      window.clearTimeout(suppressScrollTimerRef.current);
      suppressScrollTimerRef.current = null;
    }
  }, []);

  const markProgrammaticScroll = useCallback(() => {
    suppressScrollRef.current = true;

    if (suppressScrollTimerRef.current != null) {
      window.clearTimeout(suppressScrollTimerRef.current);
    }

    suppressScrollTimerRef.current = window.setTimeout(() => {
      suppressScrollRef.current = false;
      suppressScrollTimerRef.current = null;
    }, PROGRAMMATIC_SCROLL_SUPPRESS_MS);
  }, []);

  const scrollToTop = useCallback(
    (behavior: ScrollBehavior = "auto") => {
      const el = containerRef.current;
      if (!el) return;

      markProgrammaticScroll();
      el.scrollTo({ top: 0, behavior });
    },
    [markProgrammaticScroll]
  );

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "smooth") => {
      const el = containerRef.current;
      if (!el) return;

      markProgrammaticScroll();
      el.scrollTo({ top: el.scrollHeight, behavior });
    },
    [markProgrammaticScroll]
  );

  useEffect(() => {
    return () => {
      clearProgrammaticScrollMark();
    };
  }, [clearProgrammaticScrollMark]);

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

  const handleUserScrollIntent = useCallback(() => {
    if (autoScroll) {
      clearProgrammaticScrollMark();
      onAutoScrollChange(false);
    }
  }, [autoScroll, clearProgrammaticScrollMark, onAutoScrollChange]);

  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;

    const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;

    if (suppressScrollRef.current) {
      // 程序滚动已经抵达底部，可以提前解除抑制；否则继续忽略 smooth scroll 过程中的中间 scroll 事件。
      if (distanceToBottom <= 16) {
        clearProgrammaticScrollMark();
      }
      return;
    }

    if (!autoScroll) return;

    // 用户通过拖动滚动条、键盘、触控板等方式离开底部时，关闭当前页自动滚动。
    if (distanceToBottom > 16) {
      onAutoScrollChange(false);
    }
  }, [autoScroll, clearProgrammaticScrollMark, onAutoScrollChange]);

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

  return (
    <div
      id="hunting-log"
      style={{
        display: "flex",
        flexDirection: "column",
        padding: "16px 20px",
      }}
    >
      {/* ── 标题行 ── */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          marginBottom: 10,
          flexWrap: "wrap",
          gap: 6,
        }}
      >
        <h2 style={{ color: "#dcdcdc", fontSize: 16, margin: 0 }}>狩猎日志</h2>

        <span style={{ color: "#b0b0b0", fontSize: 12, marginLeft: 4 }}>
          {entries.length} 条 / {totalRounds} 轮
        </span>

        {/* 分页导航（跟随标题） */}
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
              color: "#b0b0b0",
              fontSize: 13,
              userSelect: "none",
            }}
          >
            <input
              ref={pageInputRef}
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
                fontSize: 13,
                padding: "2px 0",
                outline: "none",
              }}
            />

            <span style={{ color: "#b0b0b0" }}>/ {totalRounds}</span>
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

        {/* 自动滚动只控制当前页内部滚动 */}
        <div style={{ marginLeft: "auto" }}>
          <ToggleSwitch checked={autoScroll} onChange={onAutoScrollChange} label="自动滚动" />
        </div>
      </div>

      {/* ── 日志容器 ── */}
      <div
        ref={containerRef}
        className="log-container"
        style={{
          flex: "none",
          height: 360,
          overflowY: "auto",
          fontSize: 14,
          fontFamily: "Cascadia Code, JetBrains Mono, Consolas, monospace",
          lineHeight: 1.7,
          border: "1px solid #331e12",
          borderRadius: 4,
          padding: 10,
          background: "rgba(0,0,0,0.25)",
        }}
        onWheel={handleUserScrollIntent}
        onTouchMove={handleUserScrollIntent}
        onScroll={handleScroll}
      >
        {entries.length === 0 ? (
          <div
            style={{
              color: "#555",
              fontStyle: "italic",
              marginTop: 20,
              textAlign: "center",
            }}
          >
            等待游戏数据...
          </div>
        ) : (
          entries.map((entry, i) => {
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

      {/* ── 底部按钮行 ── */}
      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          gap: 8,
          marginTop: 10,
        }}
      >
        <button onClick={onClear} style={btnStyle}>
          清除日志
        </button>

        <button onClick={onExportAll} style={btnStyle}>
          导出全部
        </button>

        <button
          onClick={onExportCurrent}
          style={{
            ...btnStyle,
            background: "#4a2a15",
            color: "#f0d8b0",
          }}
        >
          导出本页
        </button>
      </div>
    </div>
  );
});