import { forwardRef, useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";

import { Card } from "../ui/Card";
import { SectionHeader } from "../ui/SectionHeader";
import { SectionWrapper } from "../layouts/SectionWrapper";
import { btnStyle, API } from "../constants";
import type { QuestStat, MonsterActionStats } from "../types";

function formatQuestTime(ms: number): string {
  const totalCs = Math.floor(ms / 10);
  const minutes = Math.floor(totalCs / 6000);
  const seconds = Math.floor((totalCs / 100) % 60);
  const centis = totalCs % 100;
  return `${minutes}'${String(seconds).padStart(2, "0")}'${String(centis).padStart(2, "0")}`;
}

function pct(a: number, b: number) {
  return b > 0 ? ((a / b) * 100).toFixed(1) : "0.0";
}

function buildSettlementPieOption(current: QuestStat): EChartsOption {
  const total = current.total;

  const data = [
    {
      value: current.success,
      name: "成功",
      itemStyle: { color: "#4caf50" },
    },
    {
      value: current.fail,
      name: "失败",
      itemStyle: { color: "#f44336" },
    },
    {
      value: current.abandon,
      name: "放弃",
      itemStyle: { color: "#ffa726" },
    },
  ].filter((item) => item.value > 0);

  return {
    backgroundColor: "transparent",

    legend: { show: false },

    tooltip: {
      trigger: "item",
      confine: true,
      backgroundColor: "rgba(26, 18, 12, 0.96)",
      borderColor: "#5c3a1e",
      textStyle: { color: "#f0d8b0", fontSize: 12 },
      formatter: (params) => {
        const p = params as { name?: string; value?: number };
        const value = Number(p.value ?? 0);
        return `${p.name ?? ""}<br/>${value} 场，占比 ${pct(value, total)}%`;
      },
    },

    series: [
      {
        name: "结算率",
        type: "pie",
        radius: ["0%", "68%"],
        center: ["50%", "50%"],
        animation: false,
        avoidLabelOverlap: false,
        stillShowZeroSum: false,
        label: { show: false },
        labelLine: { show: false },
        emphasis: {
          scale: false,
          label: { show: false },
        },
        blur: { label: { show: false } },
        select: { label: { show: false } },
        itemStyle: {
          borderColor: "#1b100a",
          borderWidth: 1,
        },
        data,
      },
    ],
  };
}

export const LogAnalysisSection = forwardRef<HTMLDivElement, Record<string, unknown>>(
  function LogAnalysisSection(_props, ref) {
    const [stats, setStats] = useState<QuestStat[]>([]);
    const [actionStats, setActionStats] = useState<MonsterActionStats[]>([]);
    const [selectedQuest, setSelectedQuest] = useState<string | null>(null);
    const [hoveredQuest, setHoveredQuest] = useState<string | null>(null);
    const [expandedMonster, setExpandedMonster] = useState<number | null>(null);

    // ── 轮询结算统计 ──
    useEffect(() => {
      let cancelled = false;
      const fetchStats = async () => {
        try {
          const res = await fetch(`${API}/api/quest-stats`);
          const data: QuestStat[] = await res.json();
          if (cancelled) return;
          setStats(data);
          if (data.length > 0) {
            setSelectedQuest((prev) =>
              prev && data.some((s) => s.quest_name === prev) ? prev : data[0].quest_name
            );
          }
        } catch {
          /* backend not running */
        }
      };
      fetchStats();
      const id = window.setInterval(fetchStats, 2000);
      return () => {
        cancelled = true;
        window.clearInterval(id);
      };
    }, []);

    // ── 轮询出招统计 ──
    useEffect(() => {
      let cancelled = false;
      const fetchActionStats = async () => {
        try {
          const res = await fetch(`${API}/api/action-stats`);
          const data: MonsterActionStats[] = await res.json();
          if (!cancelled) setActionStats(data);
        } catch {
          /* backend not running */
        }
      };
      fetchActionStats();
      const id = window.setInterval(fetchActionStats, 3000);
      return () => {
        cancelled = true;
        window.clearInterval(id);
      };
    }, []);

    const sortedStats = useMemo(() => {
      return [...stats].sort((a, b) => {
        if (a.total !== b.total) return b.total - a.total;
        return a.quest_name.localeCompare(b.quest_name);
      });
    }, [stats]);

    const current = sortedStats.find((s) => s.quest_name === selectedQuest);

    // 如果没有选中的 quest，用第一条（可能为空数据时的合成默认）
    const displayQuest: QuestStat = current ?? {
      quest_name: selectedQuest ?? "",
      quest_id: 0,
      total: 0,
      success: 0,
      fail: 0,
      abandon: 0,
      avg_abandon_ms: 0,
      avg_completion_ms: 0,
      fastest_ms: 0,
      slowest_ms: 0,
      recent_completions: [],
    };

    const settlementPieOption = useMemo(() => {
      return current && current.total > 0 ? buildSettlementPieOption(current) : null;
    }, [current]);

    const filterBtnStyle = (name: string): React.CSSProperties => ({
      ...btnStyle,
      background:
        selectedQuest === name ? "#5c3a1e" : hoveredQuest === name ? "#4a2a15" : "#331e12",
      color:
        selectedQuest === name ? "#ffddaa" : hoveredQuest === name ? "#f0d8b0" : "#dcdcdc",
    });

    return (
      <SectionWrapper ref={ref} id="log-analysis">
        <SectionHeader title="日志分析" description="" />

        {stats.length > 1 && (
          <div
            style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}
          >
            {sortedStats.map((s) => (
              <button
                key={s.quest_name}
                onClick={() => setSelectedQuest(s.quest_name)}
                onMouseEnter={() => setHoveredQuest(s.quest_name)}
                onMouseLeave={() => setHoveredQuest(null)}
                style={filterBtnStyle(s.quest_name)}
              >
                {s.quest_name}
              </button>
            ))}
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          {/* ── 结算率统计 ── */}
          <Card>
            <div
              style={{
                color: "#d4c090",
                fontSize: 16,
                fontWeight: 500,
                paddingBottom: 10,
                marginBottom: 12,
                borderBottom: "1px solid #331e12",
              }}
            >
              结算率统计
            </div>

            <div style={{ display: "flex", gap: 16, alignItems: "flex-start" }}>
              <div
                style={{
                  flex: 1,
                  display: "flex",
                  flexDirection: "column",
                  gap: 5,
                  minWidth: 140,
                }}
              >
                <StatRow label="总场次" value={String(displayQuest.total)} />
                <StatRow label="狩猎成功" value={String(displayQuest.success)} color="#4caf50" />
                <StatRow label="任务失败" value={String(displayQuest.fail)} color="#f44336" />
                <StatRow label="任务放弃" value={String(displayQuest.abandon)} color="#ffa726" />

                <div style={{ height: 1, background: "#331e12", margin: "4px 0" }} />

                <StatRow
                  label="狩猎成功率"
                  value={`${pct(displayQuest.success, displayQuest.total)}%`}
                  color="#4caf50"
                />
                <StatRow
                  label="任务重置率"
                  value={`${pct(displayQuest.abandon, displayQuest.total)}%`}
                  color="#ffa726"
                />

                {displayQuest.avg_abandon_ms > 0 && (
                  <StatRow
                    label="平均重置时刻"
                    value={formatQuestTime(displayQuest.avg_abandon_ms)}
                    color="#ffa726"
                  />
                )}
              </div>

              {settlementPieOption && (
                <div
                  style={{
                    width: 130,
                    flexShrink: 0,
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: 6,
                  }}
                >
                  <ReactECharts
                    option={settlementPieOption}
                    notMerge={true}
                    lazyUpdate={true}
                    style={{ width: 130, height: 130 }}
                  />
                  <div
                    style={{
                      display: "flex",
                      gap: 8,
                      flexWrap: "wrap",
                      justifyContent: "center",
                      lineHeight: 1.4,
                    }}
                  >
                    {[
                      { label: `成功 ${pct(displayQuest.success, displayQuest.total)}%`, color: "#4caf50", value: displayQuest.success },
                      { label: `失败 ${pct(displayQuest.fail, displayQuest.total)}%`, color: "#f44336", value: displayQuest.fail },
                      { label: `放弃 ${pct(displayQuest.abandon, displayQuest.total)}%`, color: "#ffa726", value: displayQuest.abandon },
                    ]
                      .filter((d) => d.value > 0)
                      .map((d) => (
                        <span
                          key={d.label}
                          style={{
                            display: "inline-flex",
                            alignItems: "center",
                            gap: 4,
                            fontSize: 11,
                            color: "#dcdcdc",
                            whiteSpace: "nowrap",
                          }}
                        >
                          <span
                            style={{
                              width: 8,
                              height: 8,
                              borderRadius: "50%",
                              backgroundColor: d.color,
                              display: "inline-block",
                              flexShrink: 0,
                            }}
                          />
                          {d.label}
                        </span>
                      ))}
                  </div>
                </div>
              )}
              {!settlementPieOption && (
                <div
                  style={{
                    width: 90,
                    flexShrink: 0,
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    justifyContent: "center",
                    color: "#6a6a6a",
                    fontSize: 12,
                    textAlign: "center",
                    lineHeight: 1.6,
                  }}
                >
                  暂无<br />任务数据
                </div>
              )}
            </div>
          </Card>

          {/* ── 结算成绩 ── */}
          <Card>
            <div
              style={{
                color: "#d4c090",
                fontSize: 16,
                fontWeight: 500,
                paddingBottom: 10,
                marginBottom: 12,
                borderBottom: "1px solid #331e12",
              }}
            >
              结算成绩
            </div>

            {displayQuest.success === 0 ? (
              <div style={{ color: "#6a6a6a", fontSize: 12, fontStyle: "italic" }}>
                暂无成功结算数据
              </div>
            ) : (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 5,
                }}
              >
                <StatRow
                  label="平均完成时间"
                  value={formatQuestTime(displayQuest.avg_completion_ms)}
                  color="#4caf50"
                />
                <StatRow
                  label="最快完成"
                  value={formatQuestTime(displayQuest.fastest_ms)}
                  color="#66bb6a"
                />
                <StatRow
                  label="最慢完成"
                  value={formatQuestTime(displayQuest.slowest_ms)}
                  color="#a5d6a7"
                />

                {displayQuest.recent_completions.length > 0 && (
                  <>
                    <div style={{ height: 1, background: "#331e12", margin: "4px 0" }} />
                    <div style={{ color: "#b8a078", fontSize: 12, marginBottom: 2 }}>
                      最近结算
                    </div>
                    <div
                      style={{
                        maxHeight: 160,
                        overflowY: "auto",
                        display: "flex",
                        flexDirection: "column",
                        gap: 2,
                      }}
                    >
                      {displayQuest.recent_completions.slice(0, 15).map((t, i) => (
                        <div
                          key={i}
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            fontSize: 12,
                            color: "#c8c8c8",
                            padding: "1px 4px",
                            borderRadius: 2,
                            background: i % 2 === 0 ? "rgba(255,255,255,0.03)" : "transparent",
                          }}
                        >
                          <span style={{ color: "#4caf50" }}>#{displayQuest.recent_completions.length - i}</span>
                          <span>{formatQuestTime(t)}</span>
                        </div>
                      ))}
                    </div>
                  </>
                )}
              </div>
            )}
          </Card>

          {/* ── 出招统计 ── */}
          <Card>
            <div style={{ color: "#d4c090", fontSize: 14, marginBottom: 6 }}>
              出招统计
            </div>
            <div style={{ color: "#8c8c8c", fontSize: 12, marginBottom: 8 }}>
              各招式使用频率统计
            </div>

            {actionStats.length === 0 ? (
              <div style={{ color: "#6a6a6a", fontSize: 12, fontStyle: "italic" }}>
                暂无出招数据
              </div>
            ) : (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                  maxHeight: 320,
                  overflowY: "auto",
                }}
              >
                {actionStats.map((m) => (
                  <div key={m.monster_id}>
                    <button
                      onClick={() =>
                        setExpandedMonster(
                          expandedMonster === m.monster_id ? null : m.monster_id
                        )
                      }
                      style={{
                        width: "100%",
                        textAlign: "left",
                        background:
                          expandedMonster === m.monster_id ? "#3a2212" : "#1e120a",
                        border: "1px solid #331e12",
                        borderRadius: 4,
                        padding: "6px 10px",
                        color: "#e0c8a0",
                        fontSize: 13,
                        cursor: "pointer",
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <span>
                        <span style={{ marginRight: 6 }}>
                          {expandedMonster === m.monster_id ? "▼" : "▶"}
                        </span>
                        {m.monster_name}
                      </span>
                      <span style={{ color: "#b8a078", fontSize: 12 }}>
                        {m.total_actions} 次出招
                      </span>
                    </button>

                    {expandedMonster === m.monster_id && (
                      <div
                        style={{
                          padding: "4px 0 4px 20px",
                          display: "flex",
                          flexDirection: "column",
                          gap: 1,
                        }}
                      >
                        {m.actions.length > 0 ? (
                          m.actions
                            .slice(0, 30)
                            .map((a) => (
                              <ActionRow
                                key={a.action_id}
                                name={a.action_name}
                                count={a.count}
                                total={m.total_actions}
                              />
                            ))
                        ) : (
                          <div style={{ color: "#6a6a6a", fontSize: 11, padding: "2px 0" }}>
                            无详细招式数据
                          </div>
                        )}
                        {m.actions.length > 30 && (
                          <div style={{ color: "#6a6a6a", fontSize: 11, padding: "2px 0" }}>
                            ...还有 {m.actions.length - 30} 个招式未显示
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </Card>

          {/* ── DPS 曲线重构 ── */}
          <Card>
            <div style={{ color: "#d4c090", fontSize: 14, marginBottom: 6 }}>
              DPS 曲线重构
            </div>
            <div style={{ color: "#8c8c8c", fontSize: 12 }}>基于时间轴的伤害输出图表</div>
            <div
              style={{
                display: "inline-block",
                marginTop: 8,
                padding: "2px 8px",
                borderRadius: 3,
                fontSize: 12,
                color: "#8c8c8c",
                border: "1px solid #555",
              }}
            >
              开发中
            </div>
          </Card>

          {/* ── 时空位置回溯 ── */}
          <Card>
            <div style={{ color: "#d4c090", fontSize: 14, marginBottom: 6 }}>
              时空位置回溯
            </div>
            <div style={{ color: "#8c8c8c", fontSize: 12 }}>
              战斗过程中怪物与玩家的时空轨迹回放
            </div>
            <div
              style={{
                display: "inline-block",
                marginTop: 8,
                padding: "2px 8px",
                borderRadius: 3,
                fontSize: 12,
                color: "#8c8c8c",
                border: "1px solid #555",
              }}
            >
              开发中
            </div>
          </Card>

          {/* ── 受击伤害统计 ── */}
          <Card>
            <div style={{ color: "#d4c090", fontSize: 14, marginBottom: 6 }}>
              受击伤害统计
            </div>
            <div style={{ color: "#8c8c8c", fontSize: 12 }}>
              每次受击的伤害来源与数值记录
            </div>
            <div
              style={{
                display: "inline-block",
                marginTop: 8,
                padding: "2px 8px",
                borderRadius: 3,
                fontSize: 12,
                color: "#8c8c8c",
                border: "1px solid #555",
              }}
            >
              开发中
            </div>
          </Card>
        </div>
      </SectionWrapper>
    );
  }
);

function StatRow({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color?: string;
}) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", gap: 16 }}>
      <span style={{ color: "#dcdcdc", fontSize: 13 }}>{label}</span>
      <span style={{ color: color ?? "#dcdcdc", fontSize: 13, fontWeight: 500 }}>
        {value}
      </span>
    </div>
  );
}

function ActionRow({
  name,
  count,
  total,
}: {
  name: string;
  count: number;
  total: number;
}) {
  const barPct = total > 0 ? (count / total) * 100 : 0;
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        fontSize: 12,
        padding: "2px 4px",
        borderRadius: 2,
        position: "relative",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          bottom: 0,
          width: `${barPct}%`,
          background: "rgba(76, 175, 80, 0.08)",
          borderRadius: 2,
          pointerEvents: "none",
        }}
      />
      <span
        style={{
          color: "#dcdcdc",
          flex: 1,
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          position: "relative",
          zIndex: 1,
        }}
      >
        {name}
      </span>
      <span
        style={{
          color: "#a5d6a7",
          fontWeight: 500,
          flexShrink: 0,
          position: "relative",
          zIndex: 1,
        }}
      >
        {count}
      </span>
    </div>
  );
}
