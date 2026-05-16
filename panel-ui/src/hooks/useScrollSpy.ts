import { useState, useCallback, useLayoutEffect, useEffect, useRef } from "react";

export type SectionId = "basic-tools" | "hunting-log" | "log-analysis" | "software-updates" | "usage-guide";

interface SectionRefs {
  basicTools: React.RefObject<HTMLDivElement | null>;
  huntingLog: React.RefObject<HTMLDivElement | null>;
  logAnalysis: React.RefObject<HTMLDivElement | null>;
  softwareUpdates: React.RefObject<HTMLDivElement | null>;
  usageGuide: React.RefObject<HTMLDivElement | null>;
}

export function useScrollSpy() {
  const [activeSection, setActiveSection] = useState<SectionId>("basic-tools");

  const mainPanelRef = useRef<HTMLDivElement>(null);

  const sectionRefs: SectionRefs = {
    basicTools: useRef<HTMLDivElement>(null),
    huntingLog: useRef<HTMLDivElement>(null),
    logAnalysis: useRef<HTMLDivElement>(null),
    softwareUpdates: useRef<HTMLDivElement>(null),
    usageGuide: useRef<HTMLDivElement>(null),
  };

  const scrollLockRef = useRef<SectionId | null>(null);
  const scrollLockTimerRef = useRef<number | null>(null);

  const resetMainPanelToTop = useCallback(() => {
    const panel = mainPanelRef.current;
    if (!panel) return;
    panel.scrollTo({ top: 0, left: 0, behavior: "auto" });
    setActiveSection("basic-tools");
  }, []);

  useLayoutEffect(() => {
    resetMainPanelToTop();
    const raf = window.requestAnimationFrame(resetMainPanelToTop);
    return () => window.cancelAnimationFrame(raf);
  }, [resetMainPanelToTop]);

  useEffect(() => {
    const panel = mainPanelRef.current;
    if (!panel) return;

    const getSectionTop = (el: HTMLElement) => el.offsetTop;

    const onScroll = () => {
      if (scrollLockRef.current) {
        setActiveSection(scrollLockRef.current);
        return;
      }

      const threshold = 80;
      const scrollTop = panel.scrollTop;
      const maxScrollTop = panel.scrollHeight - panel.clientHeight;

      if (maxScrollTop > 0 && scrollTop >= maxScrollTop - 2) {
        setActiveSection("usage-guide");
        return;
      }

      const entries: Array<{ id: SectionId; el: HTMLDivElement | null }> = [
        { id: "basic-tools", el: sectionRefs.basicTools.current },
        { id: "hunting-log", el: sectionRefs.huntingLog.current },
        { id: "log-analysis", el: sectionRefs.logAnalysis.current },
        { id: "software-updates", el: sectionRefs.softwareUpdates.current },
        { id: "usage-guide", el: sectionRefs.usageGuide.current },
      ];

      let current: SectionId = "basic-tools";
      for (const { id, el } of entries) {
        if (!el) continue;
        if (scrollTop + threshold >= getSectionTop(el)) {
          current = id;
        }
      }
      setActiveSection(current);
    };

    onScroll();
    panel.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);

    return () => {
      panel.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
      if (scrollLockTimerRef.current != null) {
        window.clearTimeout(scrollLockTimerRef.current);
        scrollLockTimerRef.current = null;
      }
    };
  }, []);

  const scrollToSection = useCallback((id: SectionId) => {
    setActiveSection(id);
    scrollLockRef.current = id;

    if (scrollLockTimerRef.current != null) {
      window.clearTimeout(scrollLockTimerRef.current);
    }
    scrollLockTimerRef.current = window.setTimeout(() => {
      scrollLockRef.current = null;
      scrollLockTimerRef.current = null;
    }, 700);

    const panel = mainPanelRef.current;
    if (!panel) return;

    const SCROLL_OFFSET = 44;
    if (id === "basic-tools") {
      panel.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }

    const target = panel.querySelector<HTMLElement>(`#${id}`);
    if (!target) return;
    panel.scrollTo({
      top: Math.max(0, target.offsetTop - SCROLL_OFFSET),
      behavior: "smooth",
    });
  }, []);

  return { activeSection, scrollToSection, mainPanelRef, sectionRefs };
}
