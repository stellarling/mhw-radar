import { lazy, Suspense } from "react";

import type { LogAnalysisProps } from "./LogAnalysisSection";

const LogAnalysisSection = lazy(() =>
  import("./LogAnalysisSection").then((module) => ({ default: module.LogAnalysisSection }))
);

export function LogAnalysis(props: LogAnalysisProps) {
  return (
    <Suspense fallback={<AnalysisPlaceholder loading />}>
      <LogAnalysisSection {...props} />
    </Suspense>
  );
}

export function AnalysisPlaceholder({ loading = false }: { loading?: boolean }) {
  return (
    <div className="min-h-24 border-0 border-t border-solid border-panel-border p-4">
      <h2 className="m-0 mb-1 text-base font-semibold text-panel-text">日志分析</h2>
      {loading && <p role="status" className="m-0 text-xs text-panel-muted">正在加载统计图表…</p>}
    </div>
  );
}
