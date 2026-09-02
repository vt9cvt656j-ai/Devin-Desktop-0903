import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "@/App";
import { configureMse, mseEnvConfig, mseReady } from "@/lib/mse";
import "@/index.css";

// 配置必须先于第一次 mseFetch 落地。晚一步的后果不是报错而是「悄悄按默认值跑」：
// 没有固定密钥、网关按同源算 —— 请求照样成功，于是这个配置错误永远不会暴露。
configureMse(mseEnvConfig());

// 预热握手，但**不挡渲染**：取公钥要一个 RTT，推导密钥还要一次 P-384 ECDH，
// await 在这里等于给控制台首屏白加一段空白。第一个请求自己会等这条 promise，
// 提前跑只是让它多半已经等完了。
void mseReady().catch(() => {
  // 引导失败不能炸掉整个应用：auto 档会退回明文，require 档会在真正发请求时抛错，
  // 两种都比一个空白页面说得清楚。
});

const container = document.getElementById("root");

if (!container) {
  throw new Error("找不到 #root 挂载点，请检查 index.html");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
