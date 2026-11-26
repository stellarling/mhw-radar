import { circleBtnStyle } from "../constants";

export function HeaderBar({
  version,
  onScrollBottom,
  onClose,
}: {
  version: string;
  onScrollBottom?: () => void;
  onClose?: (e: React.MouseEvent) => void;
}) {
  return (
    <header className={`flex min-h-11 shrink-0 items-center justify-between gap-2 bg-black/20 px-4 select-none ${onClose ? "drag-region" : ""}`}>
      <span className="text-sm text-panel-text md:text-base">
        怪物猎人世界/冰原 v15.23.00
      </span>

      <div data-no-drag className="flex shrink-0 items-center gap-2">
        <span className="text-xs text-panel-muted">version {version}</span>
        {onScrollBottom && <button type="button" data-no-drag onClick={onScrollBottom} style={circleBtnStyle} title="滚动到底部">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
            <path d="m6 9 6 6 6-6"/>
          </svg>
        </button>}
        <button disabled={!onClose} type="button" data-no-drag onClick={onClose} style={circleBtnStyle} title={onClose ? "关闭程序" : "网页演示"}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
            <path d="M18 6 6 18"/>
            <path d="m6 6 12 12"/>
          </svg>
        </button>
      </div>
    </header>
  );
}
