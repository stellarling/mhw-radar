import { LogoImage } from "../ui/LogoImage";
import { NavButton } from "../ui/NavButton";

export function Sidebar({
  activeSection,
  onNavigate,
}: {
  activeSection: string;
  onNavigate: (id: string) => void;
}) {
  return (
    <div style={{
      width: 180,
      background: "rgba(0,0,0,0.2)",
      display: "flex",
      flexDirection: "column",
      alignItems: "stretch",
      flexShrink: 0,
      paddingTop: 20,
      gap: 40,
      borderRight: "1px solid #331e12",
    }}>
      <div style={{ display: "flex", justifyContent: "center", width: "100%", padding: "0 14px" }}>
        <div style={{ width: "90%" }}>
          <LogoImage fill />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 4, width: "100%" }}>
        <NavButton
          label="基础工具"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <line x1="4" y1="6" x2="20" y2="6"/>
              <line x1="4" y1="18" x2="20" y2="18"/>
              <circle cx="12" cy="6" r="2"/>
              <circle cx="12" cy="18" r="2"/>
            </svg>
          }
          active={activeSection === "basic-tools"}
          onClick={() => onNavigate("basic-tools")}
        />
        <NavButton
          label="狩猎日志"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="16" y1="13" x2="8" y2="13"/>
              <line x1="16" y1="17" x2="8" y2="17"/>
            </svg>
          }
          active={activeSection === "hunting-log"}
          onClick={() => onNavigate("hunting-log")}
        />
        <NavButton
          label="日志分析"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="20" x2="18" y2="10"/>
              <line x1="12" y1="20" x2="12" y2="4"/>
              <line x1="6" y1="20" x2="6" y2="14"/>
            </svg>
          }
          active={activeSection === "log-analysis"}
          onClick={() => onNavigate("log-analysis")}
        />
        <NavButton
          label="软件更新"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="23 4 23 10 17 10"/>
              <polyline points="1 20 1 14 7 14"/>
              <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
            </svg>
          }
          active={activeSection === "software-updates"}
          onClick={() => onNavigate("software-updates")}
        />
        <NavButton
          label="使用说明"
          icon={
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="16" x2="12" y2="12"/>
              <line x1="12" y1="8" x2="12.01" y2="8"/>
            </svg>
          }
          active={activeSection === "usage-guide"}
          onClick={() => onNavigate("usage-guide")}
        />
      </div>
      <div style={{ marginTop: "auto", padding: "0 14px 14px", display: "flex", flexDirection: "column", gap: 2 }}>
        <span style={{ color: "#b0b0b0", fontSize: 13, lineHeight: 1.5 }}>MHW Radar</span>
        <span style={{ color: "#8c8c8c", fontSize: 12, lineHeight: 1.5 }}>数据由mhdatalab.com提供</span>
      </div>
    </div>
  );
}
