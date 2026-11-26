import { createRoot } from "react-dom/client";

import { DemoPanel } from "./DemoPanel";
import "../src/panel.css";

const root = document.getElementById("root");
if (root) createRoot(root).render(<DemoPanel />);
