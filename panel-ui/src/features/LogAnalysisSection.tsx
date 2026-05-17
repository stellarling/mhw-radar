import { forwardRef, useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";

import { Card } from "../ui/Card";
import { SectionHeader } from "../ui/SectionHeader";
import { SectionWrapper } from "../layouts/SectionWrapper";
import { btnStyle, API } from "../constants";
import type { QuestStat } from "../types";

const FEATURES = [
  { label: "结算率统计", desc: "任务完成率与狩猎效率分析" },
  { label: "DPS 曲线重构", desc: "基于时间轴的伤害输出图表" },
  { label: "出招统计", desc: "各招式使用频率与命中率" },
  { label: "时空位置回溯", desc: "战斗过程中怪物与玩家的时空轨迹回放" },
  { label: "受击伤害统计", desc: "每次受击的伤害来源与数值记录" },
];

function formatQuestTime(ms: number): string {
  const totalCs = Math.floor(ms / 10);
  const minutes = Math.floor(totalCs / 6000);
  const seconds = Math.floor((totalCs / 100) % 60);
  const centis = totalCs % 100;
  return `${minutes}'${String(seconds).padStart(2, "0")}'${String(centis).padStart(2, "0")}`;
}

function pct(a: number, b: number) {
  return b > 0 ? ((a / b) * 100).toFixed(1) : "-";
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

    // 明确关闭 ECharts 内置图例，避免和下方手写图例形成两组标签。
    legend: {
      show: false,
    },

    tooltip: {
      trigger: "item",
      confine: true,
      backgroundColor: "rgba(26, 18, 12, 0.96)",
      borderColor: "#5c3a1e",
      textStyle: {
        color: "#f0d8b0",
        fontSize: 12,
      },
      formatter: (params) => {
        const p = params as {
          name?: string;
          value?: number;
        };
        const value = Number(p.value ?? 0);
        return `${p.name ?? ""}<br/>${value} 场，占比 ${pct(value, total)}%`;
      },
    },

    series: [
      {
        name: "结算率",
        type: "pie",

        // 用实心饼图。外部标签关闭后，半径可以略大但仍留安全边距。
        radius: ["0%", "68%"],
        center: ["50%", "50%"],

        animation: false,
        avoidLabelOverlap: false,
        stillShowZeroSum: false,

        // 关键：关闭 ECharts 饼图自身标签。
        label: {
          show: false,
        },
        labelLine: {
          show: false,
        },

        // 关键：hover 时也不要弹出中心/外部标签。
        emphasis: {
          scale: false,
          label: {
            show: false,
          },
        },
        blur: {
          label: {
            show: false,
          },
        },
        select: {
          label: {
            show: false,
          },
        },

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
    const [selectedQuest, setSelectedQuest] = useState<string | null>(null);
    const [hoveredQuest, setHoveredQuest] = useState<string | null>(null);

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
          } else {
            setSelectedQuest(null);
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

    const sortedStats = useMemo(() => {
      return [...stats].sort((a, b) => {
        if (a.total !== b.total) return b.total - a.total;
        return a.quest_name.localeCompare(b.quest_name);
      });
    }, [stats]);

    const current = sortedStats.find((s) => s.quest_name === selectedQuest);

    const settlementPieOption = useMemo(() => {
      return current && current.total > 0 ? buildSettlementPieOption(current) : null;
    }, [current]);

    const filterBtnStyle = (name: string): React.CSSProperties => ({
      ...btnStyle,
      background:
        selectedQuest === name
          ? "#5c3a1e"
          : hoveredQuest === name
            ? "#4a2a15"
            : "#331e12",
      color:
        selectedQuest === name
          ? "#ffddaa"
          : hoveredQuest === name
            ? "#f0d8b0"
            : "#dcdcdc",
    });

    return (
      <SectionWrapper ref={ref} id="log-analysis">
        <SectionHeader title="日志分析" description="开发中" />

        {stats.length > 1 && (
          <div
            style={{
              display: "flex",
              gap: 8,
              flexWrap: "wrap",
              marginBottom: 12,
            }}
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

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
          }}
        >
          {FEATURES.map((item, idx) => {
            if (idx === 0) {
              return (
                <Card key={item.label} style={{ gridColumn: "1 / -1" }}>
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
                    {item.label}
                  </div>

                  {!current || current.total === 0 || !settlementPieOption ? (
                    <div
                      style={{
                        color: "#8c8c8c",
                        fontSize: 12,
                        fontStyle: "italic",
                      }}
                    >
                      暂无任务记录
                    </div>
                  ) : (
                    <div
                      style={{
                        display: "flex",
                        gap: 24,
                        alignItems: "center",
                      }}
                    >
                      <div
                        style={{
                          flex: 1,
                          display: "flex",
                          flexDirection: "column",
                          gap: 6,
                          minWidth: 180,
                        }}
                      >
                        <StatRow label="总场次" value={String(current.total)} />
                        <StatRow
                          label="狩猎成功"
                          value={String(current.success)}
                          color="#4caf50"
                        />
                        <StatRow
                          label="任务失败"
                          value={String(current.fail)}
                          color="#f44336"
                        />
                        <StatRow
                          label="任务放弃"
                          value={String(current.abandon)}
                          color="#ffa726"
                        />

                        <div
                          style={{
                            height: 1,
                            background: "#331e12",
                            margin: "4px 0",
                          }}
                        />

                        <StatRow
                          label="狩猎成功率"
                          value={`${pct(current.success, current.total)}%`}
                          color="#4caf50"
                        />
                        <StatRow
                          label="任务重置率"
                          value={`${pct(current.abandon, current.total)}%`}
                          color="#ffa726"
                        />

                        {current.avg_abandon_ms > 0 && (
                          <StatRow
                            label="平均重置时刻"
                            value={formatQuestTime(current.avg_abandon_ms)}
                            color="#ffa726"
                          />
                        )}
                      </div>

                      <div
                        style={{
                          width: 170,
                          flexShrink: 0,
                          display: "flex",
                          flexDirection: "column",
                          alignItems: "center",
                          gap: 8,
                        }}
                      >
                        <ReactECharts
                          option={settlementPieOption}
                          notMerge={true}
                          lazyUpdate={true}
                          style={{
                            width: 150,
                            height: 150,
                          }}
                        />

                        <div
                          style={{
                            display: "flex",
                            gap: 10,
                            flexWrap: "wrap",
                            justifyContent: "center",
                            lineHeight: 1.4,
                          }}
                        >
                          {[
                            {
                              label: `成功 ${pct(current.success, current.total)}%`,
                              color: "#4caf50",
                              value: current.success,
                            },
                            {
                              label: `失败 ${pct(current.fail, current.total)}%`,
                              color: "#f44336",
                              value: current.fail,
                            },
                            {
                              label: `放弃 ${pct(current.abandon, current.total)}%`,
                              color: "#ffa726",
                              value: current.abandon,
                            },
                          ]
                            .filter((d) => d.value > 0)
                            .map((d) => (
                              <span
                                key={d.label}
                                style={{
                                  display: "inline-flex",
                                  alignItems: "center",
                                  gap: 4,
                                  fontSize: 12,
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
                                  }}
                                />
                                {d.label}
                              </span>
                            ))}
                        </div>
                      </div>
                    </div>
                  )}
                </Card>
              );
            }

            return (
              <Card key={item.label}>
                <div style={{ color: "#d4c090", fontSize: 14, marginBottom: 6 }}>
                  {item.label}
                </div>
                <div style={{ color: "#8c8c8c", fontSize: 12 }}>{item.desc}</div>
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
            );
          })}
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