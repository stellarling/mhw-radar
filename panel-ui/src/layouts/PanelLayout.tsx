import type { ComponentProps, ReactNode, Ref } from "react";

import { HeaderBar } from "./HeaderBar";
import { MainLayout } from "./MainLayout";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";

interface PanelLayoutProps {
  sidebar: ComponentProps<typeof Sidebar>;
  header: ComponentProps<typeof HeaderBar>;
  status: ComponentProps<typeof StatusBar>;
  contentRef?: Ref<HTMLDivElement>;
  children: ReactNode;
}

export function PanelLayout({ sidebar, header, status, contentRef, children }: PanelLayoutProps) {
  return (
    <MainLayout>
      <Sidebar {...sidebar} />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <HeaderBar {...header} />
        <main ref={contentRef} className="main-panel relative min-h-0 flex-1 overflow-y-auto">
          <StatusBar {...status} />
          {children}
        </main>
      </div>
    </MainLayout>
  );
}
