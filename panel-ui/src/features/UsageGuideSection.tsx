import { forwardRef } from "react";
import { Card } from "../ui/Card";
import { SectionHeader } from "../ui/SectionHeader";
import { SectionWrapper } from "../layouts/SectionWrapper";

export const UsageGuideSection = forwardRef<HTMLDivElement, Record<string, unknown>>(function UsageGuideSection(_props, ref) {
  return (
    <SectionWrapper ref={ref} id="usage-guide">
      <SectionHeader title="使用说明" description="关于本软件与相关声明" />

      <Card style={{ marginBottom: 16 }}>
        <div style={{ color: "#bfa76b", fontSize: 15, marginBottom: 8 }}>关于本软件</div>
        <div style={{ color: "#d0d0d0", fontSize: 14, lineHeight: 1.8 }}>
          <div style={{ marginBottom: 10 }}>
            MHW Radar 是一款为《怪物猎人：世界》玩家打造的实时狩猎辅助工具。
            目前还在早期的开发阶段，请谨慎使用。
            通过读取内存数据并输出到可视化透明覆盖层，在游戏画面中实时显示怪物血量、位置距离、
            攻击角度、伤害数值及任务进度等信息，帮助猎人更精准地掌握战局、优化输出节奏。
          </div>

          <div style={{ marginBottom: 10 }}>
            但事实上我们并不鼓励开启悬浮窗，本软件更合适的用途是用于战斗的复盘和更好得狩猎，因此软件
            内置完整的狩猎日志系统，自动记录每次攻击与怪物动作变更，支持导出分析与即时复盘。
          </div>

          <div>
            本工具仅提供信息呈现，不修改任何游戏文件或内存数据，所有解析均基于客户端只读方式，
            不影响游戏原有逻辑与网络通信，符合公平游戏原则。
          </div>
        </div>
      </Card>

      <Card style={{ background: "rgba(0,0,0,0.08)" }}>
        <div style={{ color: "#b0b0b0", fontSize: 13, lineHeight: 2 }}>
          本工具为开源免费软件，仅供学习交流使用，禁止用于商业用途。<br />
          所有游戏相关数据、美术素材及商标版权均归属 CAPCOM CO., LTD.<br />
          《怪物猎人：世界》《怪物猎人：世界·冰原》© CAPCOM CO., LTD. ALL RIGHTS RESERVED.<br />
          使用本工具所产生的任何后果由使用者自行承担。
        </div>
      </Card>
    </SectionWrapper>
  );
});
