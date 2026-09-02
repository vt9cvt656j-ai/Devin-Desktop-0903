import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { makeDom, Ev, Element_ } from "./dom.mjs";

const IDE = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const doc = makeDom(readFileSync(join(IDE, "index.html"), "utf8"));

// The production shell is React now, so index.html intentionally contains only
// #root. main.js still queries its controls synchronously at module load. This
// harness exercises the executor, not React rendering, therefore provide the
// shell's declared ID contract before importing main.js. Keeping this derived
// from Shell.jsx prevents the fixture from drifting whenever a control moves.
const shellSource = readFileSync(join(IDE, "src", "app", "Shell.jsx"), "utf8");
for (const match of shellSource.matchAll(/\bid="([^"]+)"/g)) {
  const id = match[1];
  if (doc.getElementById(id)) continue;
  const element = doc.createElement("div");
  element.id = id;
  doc.body.appendChild(element);
}

// `main.js` synchronously reads the model glyph through
// `modelPickerBtn.querySelector("use")`.  The generic ID loop above only
// creates an empty host, while Shell.jsx declares an SVG/use pair inside this
// button. Keep the fixture to that observable shell contract rather than
// duplicating the full composer tree.
const modelPickerBtn = doc.getElementById("modelPickerBtn");
if (modelPickerBtn && !modelPickerBtn.querySelector("use")) {
  const icon = doc.createElement("svg");
  icon.className = "ic";
  const use = doc.createElement("use");
  use.setAttribute("href", "#i-cpu");
  icon.appendChild(use);
  modelPickerBtn.appendChild(icon);
}

class Store { constructor(){ this.m=new Map(); }
  getItem(k){ return this.m.has(String(k)) ? this.m.get(String(k)) : null; }
  setItem(k,v){ this.m.set(String(k), String(v)); }
  removeItem(k){ this.m.delete(String(k)); }
  clear(){ this.m.clear(); }
  key(i){ return [...this.m.keys()][i] ?? null; }
  get length(){ return this.m.size; }
}

const listeners = new Map();
const win = {
  document: doc,
  location: { href:"http://localhost/", origin:"http://localhost", protocol:"http:", host:"localhost", hostname:"localhost", port:"", pathname:"/", search:"", hash:"", reload(){}, assign(){}, replace(){}, toString(){return "http://localhost/";} },
  navigator: { platform:"MacIntel", userAgent:"Node/E2E", language:"en-US", languages:["en-US"], clipboard:{ writeText:async()=>{}, readText:async()=>"" }, onLine:true, maxTouchPoints:0, hardwareConcurrency:8, permissions:{ query:async()=>({state:"granted", addEventListener(){}}) }, mediaDevices:{ getUserMedia:async()=>({ getTracks:()=>[] }) }, sendBeacon(){return true;}, storage:{ estimate:async()=>({usage:0,quota:0}) } },
  localStorage: new Store(),
  sessionStorage: new Store(),
  innerWidth: 1440, innerHeight: 900, outerWidth: 1440, outerHeight: 900, devicePixelRatio: 2,
  scrollX:0, scrollY:0, pageXOffset:0, pageYOffset:0,
  addEventListener(t,f){ if(!listeners.has(t)) listeners.set(t,new Set()); listeners.get(t).add(f); },
  removeEventListener(t,f){ listeners.get(t)?.delete(f); },
  dispatchEvent(e){ for(const f of [...(listeners.get(e.type)||[])]) { try{ f(e); }catch(err){ console.error("[win] listener threw:", err.message); } } return true; },
  matchMedia(q){ return { matches:false, media:q, addEventListener(){}, removeEventListener(){}, addListener(){}, removeListener(){}, onchange:null }; },
  getComputedStyle(){ return new Proxy({ getPropertyValue:()=> "" }, { get(t,p){ return p in t ? t[p] : ""; } }); },
  requestAnimationFrame(cb){ return setTimeout(()=>cb(Date.now()), 0); },
  cancelAnimationFrame(id){ clearTimeout(id); },
  requestIdleCallback(cb){ return setTimeout(()=>cb({ didTimeout:false, timeRemaining:()=>0 }), 0); },
  cancelIdleCallback(id){ clearTimeout(id); },
  open(){ return null; }, close(){}, focus(){}, blur(){}, print(){}, alert(){}, confirm(){return false;}, prompt(){return null;},
  scrollTo(){}, scrollBy(){},
  getSelection(){ return doc.getSelection(); },
  crypto: globalThis.crypto,
  performance: globalThis.performance,
  isSecureContext: true,
  name: "",
  frames: [], parent: null, top: null, self: null,
};
win.self = win; win.parent = win; win.top = win; win.window = win; win.globalThis = win;

class CustomEvent_ extends Ev { constructor(t,i={}){ super(t,i); this.detail=i.detail; } }

const G = {
  window: win, document: doc, self: win, navigator: win.navigator, location: win.location,
  localStorage: win.localStorage, sessionStorage: win.sessionStorage,
  Event: Ev, CustomEvent: CustomEvent_, MouseEvent: Ev, KeyboardEvent: Ev, PointerEvent: Ev, DragEvent: Ev, InputEvent: Ev, FocusEvent: Ev, WheelEvent: Ev, ErrorEvent: Ev, PromiseRejectionEvent: Ev, MessageEvent: Ev, CloseEvent: Ev, ProgressEvent: Ev, TouchEvent: Ev, ClipboardEvent: Ev, CompositionEvent: Ev, StorageEvent: Ev, PopStateEvent: Ev, HashChangeEvent: Ev, AnimationEvent: Ev, TransitionEvent: Ev,
  Element: Element_, HTMLElement: Element_, HTMLInputElement: Element_, HTMLTextAreaElement: Element_, HTMLCanvasElement: Element_, HTMLImageElement: Element_, HTMLIFrameElement: Element_, HTMLSelectElement: Element_, HTMLAnchorElement: Element_, Node: Element_, SVGElement: Element_,
  requestAnimationFrame: win.requestAnimationFrame, cancelAnimationFrame: win.cancelAnimationFrame,
  requestIdleCallback: win.requestIdleCallback, cancelIdleCallback: win.cancelIdleCallback,
  matchMedia: win.matchMedia, getComputedStyle: win.getComputedStyle,
  alert: win.alert, confirm: win.confirm, prompt: win.prompt,
  getSelection: win.getSelection,
  devicePixelRatio: 2, innerWidth: 1440, innerHeight: 900,
  ResizeObserver: class { observe(){} unobserve(){} disconnect(){} },
  MutationObserver: class { constructor(cb){this.cb=cb;} observe(){} disconnect(){} takeRecords(){return [];} },
  IntersectionObserver: class { constructor(cb){this.cb=cb;} observe(){} unobserve(){} disconnect(){} takeRecords(){return [];} },
  PerformanceObserver: class { observe(){} disconnect(){} },
  Image: class { constructor(){ this.onload=null; this.onerror=null; this.src=""; } addEventListener(){} removeEventListener(){} },
  Audio: class { play(){return Promise.resolve();} pause(){} addEventListener(){} },
  Worker: class { constructor(){ this.onmessage=null; } postMessage(){} terminate(){} addEventListener(){} removeEventListener(){} },
  SharedWorker: class { constructor(){ this.port={postMessage(){},start(){},addEventListener(){}}; } },
  WebSocket: class { constructor(url){ this.url=url; this.readyState=0; this.onopen=this.onmessage=this.onerror=this.onclose=null; if(process.env.E2E_ALLOW_WS!=="1") { /* inert */ } } send(){} close(){} addEventListener(){} removeEventListener(){} static CONNECTING=0; static OPEN=1; static CLOSING=2; static CLOSED=3; },
  EventSource: class { constructor(){ this.onmessage=null; } close(){} addEventListener(){} },
  XMLHttpRequest: class { open(){} send(){} setRequestHeader(){} abort(){} addEventListener(){} },
  FileReader: class { readAsText(){} readAsDataURL(){} readAsArrayBuffer(){} addEventListener(){} },
  DOMParser: class { parseFromString(s){ const d=makeDom(""); return d; } },
  XMLSerializer: class { serializeToString(){ return ""; } },
  CSS: { supports:()=>false, escape:(s)=>String(s) },
  ClipboardItem: class {},
  IDBKeyRange: {}, indexedDB: { open(){ return { onsuccess:null, onerror:null, onupgradeneeded:null }; }, deleteDatabase(){ return {}; } },
  speechSynthesis: { speak(){}, cancel(){}, getVoices(){return [];} },
  scrollTo(){}, scrollBy(){}, scroll(){},
};

export function installGlobals(){
  // FORCE-override. Node 24 ships native WebSocket/fetch/Event/etc; leaving them in
  // place makes main.js dial out for real at import time.
  for (const [k,v] of Object.entries(G)) {
    try { Object.defineProperty(globalThis, k, { value:v, writable:true, configurable:true }); }
    catch (e) { console.error("[globals] cannot define", k, e.message); }
  }
  return { win, doc };
}
export { win, doc };
