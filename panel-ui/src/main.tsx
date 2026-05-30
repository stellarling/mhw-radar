import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// 标记 WebView/React 启动时间点
performance.mark("react-boot");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// 首帧渲染完成
performance.mark("react-mounted");
const measure = performance.measure("boot-to-mounted", "react-boot", "react-mounted");
console.log(`[startup] React boot to mounted: ${measure.duration.toFixed(0)}ms`);
