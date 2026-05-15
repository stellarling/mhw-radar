import { circleBtnStyle } from "../constants";

export function HeaderBar({
  onScrollBottom,
  onClose,
}: {
  onScrollBottom: () => void;
  onClose: (e: React.MouseEvent) => void;
}) {
  return (
    <div
      className="drag-region"
      style={{
        display: "flex",
        alignItems: "center",
        padding: "0 20px",
        minHeight: 44,
        userSelect: "none",
        background: "rgba(0,0,0,0.2)",
      }}
    >
      <span style={{ color: "#dcdcdc", fontSize: 16, whiteSpace: "nowrap" }}>
        怪物猎人世界/冰原 v15.23.00
      </span>

      <div data-no-drag style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 8 }}>
        <button data-no-drag onClick={onScrollBottom} style={circleBtnStyle} title="滚动到底部">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
            <path d="m6 9 6 6 6-6"/>
          </svg>
        </button>
        <button data-no-drag onClick={onClose} style={circleBtnStyle} title="关闭程序">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
            <path d="M18 6 6 18"/>
            <path d="m6 6 12 12"/>
          </svg>
        </button>
      </div>
    </div>
  );
}
