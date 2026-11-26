import { useEffect, useState, type ComponentProps, type ReactNode } from "react";

import { PanelLayout } from "../layouts/PanelLayout";
import { OpacitySection } from "../features/OpacitySection";
import { ToggleSection } from "../features/ToggleSection";
import { ConnectionDiagnostics } from "../features/ConnectionDiagnostics";
import { LogSection } from "../features/LogSection";
import { AnalysisPlaceholder } from "../features/LogAnalysis";
import { SoftwareUpdatesSection } from "../features/SoftwareUpdatesSection";
import { UsageGuideSection } from "../features/UsageGuideSection";
import { useScrollSpy } from "../hooks/useScrollSpy";
import { StatusBar } from "./StatusBar";

interface PanelProps {
  version: string;
  logo: ReactNode;
  onOpenExternal?: (url: string) => void;
  onClose?: (event: React.MouseEvent) => void;
  status: ComponentProps<typeof StatusBar>;
  banner?: ReactNode;
  opacity: ComponentProps<typeof OpacitySection>;
  toggles: ComponentProps<typeof ToggleSection>;
  diagnostics: ComponentProps<typeof ConnectionDiagnostics>;
  logs: ComponentProps<typeof LogSection>;
  analysis: ReactNode;
  updates: ComponentProps<typeof SoftwareUpdatesSection>;
}

export function PanelView({ version, logo, onOpenExternal, onClose, status, banner, opacity, toggles, diagnostics, logs, analysis, updates }: PanelProps) {
  const { activeSection, scrollToSection, mainPanelRef, sectionRefs } = useScrollSpy();
  const [analysisVisited, setAnalysisVisited] = useState(false);

  useEffect(() => {
    if (activeSection === "log-analysis") setAnalysisVisited(true);
  }, [activeSection]);

  const scrollToBottom = () => {
    mainPanelRef.current?.scrollTo({
      top: mainPanelRef.current.scrollHeight,
      behavior: "smooth",
    });
  };

  return (
    <PanelLayout
      sidebar={{ activeSection, onNavigate: scrollToSection, logo, onOpenExternal }}
      header={{ version, onScrollBottom: scrollToBottom, onClose }}
      status={status}
      contentRef={mainPanelRef}
    >
      {banner}
      <OpacitySection {...opacity} />

      <div ref={sectionRefs.basicTools} id="basic-tools">
        <ToggleSection {...toggles} />

        <ConnectionDiagnostics {...diagnostics} />
      </div>

      <div ref={sectionRefs.huntingLog} id="hunting-log" className="border-0 border-t border-solid border-panel-border">
        <LogSection {...logs} />
      </div>

      <div ref={sectionRefs.logAnalysis} id="log-analysis">
        {analysisVisited ? analysis : <AnalysisPlaceholder />}
      </div>

      <SoftwareUpdatesSection ref={sectionRefs.softwareUpdates} {...updates} />

      <UsageGuideSection ref={sectionRefs.usageGuide} />
    </PanelLayout>
  );
}