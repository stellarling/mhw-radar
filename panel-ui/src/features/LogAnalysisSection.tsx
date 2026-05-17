import { forwardRef } from "react";
import { Card } from "../ui/Card";
import { SectionHeader } from "../ui/SectionHeader";
import { SectionWrapper } from "../layouts/SectionWrapper";

const FEATURES = [
  { label: "DPS 曲线重构", desc: "基于时间轴的伤害输出图表" },
  { label: "出招统计", desc: "各招式使用频率与命中率" },
  { label: "结算率统计", desc: "任务完成率与狩猎效率分析" },
  { label: "时空位置回溯", desc: "战斗过程中怪物与玩家的时空轨迹回放" },
  { label: "受击伤害统计", desc: "每次受击的伤害来源与数值记录" },
];

export const LogAnalysisSection = forwardRef<HTMLDivElement, Record<string, unknown>>(function LogAnalysisSection(_props, ref) {
  return (
    <SectionWrapper ref={ref} id="log-analysis">
      <SectionHeader title="日志分析" description="开发中，敬请期待" />

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
        {FEATURES.map((item) => (
          <Card key={item.label}>
            <div style={{ color: "#bfa76b", fontSize: 14, marginBottom: 6 }}>{item.label}</div>
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
        ))}
      </div>
    </SectionWrapper>
  );
});
