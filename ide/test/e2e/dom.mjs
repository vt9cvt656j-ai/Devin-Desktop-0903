// Minimal DOM good enough to import src/main.js in Node.
const VOID = new Set(["area","base","br","col","embed","hr","img","input","link","meta","param","source","track","wbr"]);

class ClassList {
  constructor(el){ this.el = el; this._s = new Set(); }
  add(...c){ c.forEach(x=>x&&this._s.add(x)); }
  remove(...c){ c.forEach(x=>this._s.delete(x)); }
  toggle(c,f){ const has=this._s.has(c); const on = f===undefined ? !has : !!f; on?this._s.add(c):this._s.delete(c); return on; }
  contains(c){ return this._s.has(c); }
  replace(a,b){ if(this._s.delete(a)){ this._s.add(b); return true;} return false; }
  get value(){ return [...this._s].join(" "); }
  get length(){ return this._s.size; }
  item(i){ return [...this._s][i] ?? null; }
  [Symbol.iterator](){ return this._s[Symbol.iterator](); }
  toString(){ return this.value; }
}

class Ev {
  constructor(type, init={}){ this.type=type; Object.assign(this,init); this.defaultPrevented=false; this.target=null; this.currentTarget=null; this.bubbles=!!init.bubbles; }
  preventDefault(){ this.defaultPrevented=true; }
  stopPropagation(){ this._stop=true; }
  stopImmediatePropagation(){ this._stop=true; }
}

class Node_ {
  constructor(){ this.childNodes=[]; this.parentNode=null; this._lis=new Map(); }
  get children(){ return this.childNodes.filter(n=>n.nodeType===1); }
  get parentElement(){ return this.parentNode && this.parentNode.nodeType===1 ? this.parentNode : null; }
  get ownerDocument(){ let n=this; while(n.parentNode) n=n.parentNode; return n.nodeType===9?n:globalThis.document; }
  get firstChild(){ return this.childNodes[0]||null; }
  get lastChild(){ return this.childNodes[this.childNodes.length-1]||null; }
  get firstElementChild(){ return this.children[0]||null; }
  get lastElementChild(){ const c=this.children; return c[c.length-1]||null; }
  get childElementCount(){ return this.children.length; }
  get nextSibling(){ const p=this.parentNode; if(!p) return null; return p.childNodes[p.childNodes.indexOf(this)+1]||null; }
  get previousSibling(){ const p=this.parentNode; if(!p) return null; return p.childNodes[p.childNodes.indexOf(this)-1]||null; }
  get nextElementSibling(){ const p=this.parentNode; if(!p) return null; const c=p.children; return c[c.indexOf(this)+1]||null; }
  get previousElementSibling(){ const p=this.parentNode; if(!p) return null; const c=p.children; return c[c.indexOf(this)-1]||null; }
  appendChild(n){ if(!n) return n; if(n.nodeType===11){ [...n.childNodes].forEach(c=>this.appendChild(c)); return n; } if(n.parentNode) n.parentNode.removeChild(n); n.parentNode=this; this.childNodes.push(n); _index(n); return n; }
  append(...ns){ ns.forEach(n=>this.appendChild(typeof n==="string"?new Text_(n):n)); }
  after(...ns){ const p=this.parentNode; if(!p) return; let ref=this.nextSibling; ns.forEach(n=>p.insertBefore(typeof n==="string"?new Text_(n):n, ref)); }
  before(...ns){ const p=this.parentNode; if(!p) return; ns.forEach(n=>p.insertBefore(typeof n==="string"?new Text_(n):n, this)); }
  replaceWith(...ns){ this.after(...ns); this.remove(); }
  prepend(...ns){ ns.reverse().forEach(n=>this.insertBefore(typeof n==="string"?new Text_(n):n, this.firstChild)); }
  insertBefore(n, ref){ if(!ref) return this.appendChild(n); const i=this.childNodes.indexOf(ref); if(i<0) return this.appendChild(n); if(n.parentNode) n.parentNode.removeChild(n); n.parentNode=this; this.childNodes.splice(i,0,n); _index(n); return n; }
  removeChild(n){ const i=this.childNodes.indexOf(n); if(i>=0){ this.childNodes.splice(i,1); n.parentNode=null; } return n; }
  remove(){ if(this.parentNode) this.parentNode.removeChild(this); }
  replaceChild(nu,old){ const i=this.childNodes.indexOf(old); if(i<0) return old; nu.parentNode=this; this.childNodes[i]=nu; old.parentNode=null; _index(nu); return old; }
  replaceChildren(...ns){ this.childNodes.forEach(c=>c.parentNode=null); this.childNodes=[]; ns.forEach(n=>this.appendChild(typeof n==="string"?new Text_(n):n)); }
  contains(n){ while(n){ if(n===this) return true; n=n.parentNode; } return false; }
  cloneNode(deep){ const el=new Element_(this.tagName||"div"); if(this.attributes) for(const [k,v] of Object.entries(this._attrs||{})) el.setAttribute(k,v); if(deep) this.childNodes.forEach(c=>el.appendChild(c.cloneNode(true))); return el; }
  addEventListener(t,f){ if(!f) return; if(!this._lis.has(t)) this._lis.set(t,new Set()); this._lis.get(t).add(f); }
  removeEventListener(t,f){ this._lis.get(t)?.delete(f); }
  dispatchEvent(e){ e.target=e.target||this; let n=this; while(n){ e.currentTarget=n; for(const f of [...(n._lis.get(e.type)||[])]){ try{ typeof f==="function"?f.call(n,e):f.handleEvent?.(e); }catch(err){ console.error("[dom] listener threw:",err.message); } if(e._stop) break; } if(e._stop||!e.bubbles) break; n=n.parentNode; } return !e.defaultPrevented; }
}

class Text_ extends Node_ { constructor(t){ super(); this.nodeType=3; this.nodeValue=String(t??""); } get textContent(){return this.nodeValue;} set textContent(v){this.nodeValue=String(v??"");} get data(){return this.nodeValue;} set data(v){this.nodeValue=String(v??"");} cloneNode(){ return new Text_(this.nodeValue); } }
class Frag_ extends Node_ { constructor(){ super(); this.nodeType=11; } }

let _doc = null;
function _index(n){ if(!_doc) return; if(n.nodeType===1){ if(n.id) _doc._ids.set(n.id,n); n.children.forEach(_index); } }

class Element_ extends Node_ {
  constructor(tag){
    super();
    this.nodeType=1;
    this.tagName=String(tag||"div").toUpperCase();
    this.localName=String(tag||"div").toLowerCase();
    this._attrs={};
    this.classList=new ClassList(this);
    this.style=new Proxy({ setProperty(k,v){ this[k]=v; }, removeProperty(k){ delete this[k]; }, getPropertyValue(k){ return this[k]??""; }, cssText:"" },{ get(t,p){ return p in t ? t[p] : ""; }, set(t,p,v){ t[p]=v; return true; } });
    this.dataset={};
    this.scrollTop=0; this.scrollLeft=0; this.scrollHeight=0; this.scrollWidth=0;
    this.clientHeight=0; this.clientWidth=0; this.offsetHeight=0; this.offsetWidth=0; this.offsetTop=0; this.offsetLeft=0;
    this.value=""; this.checked=false; this.disabled=false; this.hidden=false; this.selectedIndex=-1;
    this.selectionStart=0; this.selectionEnd=0; this.readOnly=false; this.open=false; this.files=[];
    this.isConnected=true;
  }
  get id(){ return this._attrs.id||""; } set id(v){ this._attrs.id=String(v); if(_doc) _doc._ids.set(String(v),this); }
  get className(){ return this.classList.value; } set className(v){ this.classList._s=new Set(String(v||"").split(/\s+/).filter(Boolean)); }
  get attributes(){ return Object.entries(this._attrs).map(([name,value])=>({name,value})); }
  setAttribute(k,v){ this._attrs[k]=String(v); if(k==="id"&&_doc) _doc._ids.set(String(v),this); if(k==="class") this.className=v; if(k.startsWith("data-")) this.dataset[k.slice(5).replace(/-([a-z])/g,(m,c)=>c.toUpperCase())]=String(v); }
  getAttribute(k){ return k==="class" ? (this.classList.value||null) : (k in this._attrs ? this._attrs[k] : null); }
  hasAttribute(k){ return k==="class" ? this.classList.length>0 : k in this._attrs; }
  removeAttribute(k){ delete this._attrs[k]; if(k==="class") this.classList._s=new Set(); }
  toggleAttribute(k,f){ const has=this.hasAttribute(k); const on=f===undefined?!has:!!f; on?this.setAttribute(k,""):this.removeAttribute(k); return on; }
  get textContent(){ return this.childNodes.map(n=>n.textContent??"").join(""); }
  set textContent(v){ this.childNodes.forEach(c=>c.parentNode=null); this.childNodes=[]; if(v!=null&&v!=="") this.appendChild(new Text_(v)); }
  get innerText(){ return this.textContent; } set innerText(v){ this.textContent=v; }
  get innerHTML(){ return this._html ?? this.textContent; }
  set innerHTML(v){ this.childNodes.forEach(c=>c.parentNode=null); this.childNodes=[]; this._html=String(v??""); const parsed=parseHTML(String(v??"")); parsed.forEach(n=>this.appendChild(n)); }
  get outerHTML(){ return `<${this.localName}>${this.innerHTML}</${this.localName}>`; }
  insertAdjacentHTML(pos,html){ const nodes=parseHTML(String(html||"")); if(pos==="beforeend") nodes.forEach(n=>this.appendChild(n)); else if(pos==="afterbegin") nodes.reverse().forEach(n=>this.insertBefore(n,this.firstChild)); else if(pos==="beforebegin"&&this.parentNode) nodes.forEach(n=>this.parentNode.insertBefore(n,this)); else if(pos==="afterend"&&this.parentNode) nodes.forEach(n=>this.parentNode.insertBefore(n,this.nextSibling)); }
  insertAdjacentElement(pos,el){ if(pos==="beforeend") this.appendChild(el); else if(pos==="afterbegin") this.insertBefore(el,this.firstChild); else if(pos==="beforebegin"&&this.parentNode) this.parentNode.insertBefore(el,this); else if(pos==="afterend"&&this.parentNode) this.parentNode.insertBefore(el,this.nextSibling); return el; }
  matches(sel){ return _matches(this,sel); }
  closest(sel){ let n=this; while(n&&n.nodeType===1){ if(_matches(n,sel)) return n; n=n.parentNode; } return null; }
  querySelector(sel){ return _qsa(this,sel)[0]||null; }
  querySelectorAll(sel){ const r=_qsa(this,sel); r.forEach=Array.prototype.forEach.bind(r); r.item=(i)=>r[i]??null; return r; }
  getElementsByClassName(c){ return _qsa(this,"."+c); }
  getElementsByTagName(t){ return _qsa(this,t); }
  focus(){ if(_doc) _doc.activeElement=this; } blur(){ if(_doc&&_doc.activeElement===this) _doc.activeElement=_doc.body; }
  click(){ this.dispatchEvent(new Ev("click",{bubbles:true})); }
  scrollIntoView(){} scrollTo(){} scrollBy(){} select(){} setSelectionRange(){}
  getBoundingClientRect(){ return { x:0,y:0,top:0,left:0,right:0,bottom:0,width:0,height:0,toJSON(){return{};} }; }
  getClientRects(){ return []; }
  animate(){ return { finished:Promise.resolve(), cancel(){}, play(){}, pause(){} }; }
  attachShadow(){ return this; }
  setPointerCapture(){} releasePointerCapture(){} hasPointerCapture(){return false;}
  requestFullscreen(){ return Promise.resolve(); }
  showPicker(){}
  play(){ return Promise.resolve(); } pause(){}
  getContext(){ return null; }
}

// ---- tiny selector engine: supports "tag", "#id", ".cls", "[attr]", "[a=v]", combinations,
// descendant space, ">" child, and "," lists. Enough for main.js.
function _parseSimple(s){
  const out={ tag:null, id:null, cls:[], attrs:[] };
  const re=/([#.]?[\w-]+|\[[^\]]+\]|\*|:[\w-]+(\([^)]*\))?)/g; let m;
  while((m=re.exec(s))){
    const tok=m[1];
    if(tok==="*") continue;
    else if(tok[0]==="#") out.id=tok.slice(1);
    else if(tok[0]===".") out.cls.push(tok.slice(1));
    else if(tok[0]==="[") { const inner=tok.slice(1,-1); const mm=/^([\w-]+)(?:([~^$*|]?=)"?'?([^"']*)"?'?)?$/.exec(inner); if(mm) out.attrs.push([mm[1],mm[2],mm[3]]); }
    else if(tok[0]===":") out.pseudo=tok;
    else out.tag=tok.toLowerCase();
  }
  return out;
}
function _matchSimple(el,p){
  if(el.nodeType!==1) return false;
  if(p.tag && el.localName!==p.tag) return false;
  if(p.id && el.id!==p.id) return false;
  for(const c of p.cls) if(!el.classList.contains(c)) return false;
  for(const [k,op,v] of p.attrs){ const av=el.getAttribute(k); if(av==null) return false; if(op==="=" && av!==v) return false; if(op==="*=" && !av.includes(v)) return false; if(op==="^=" && !av.startsWith(v)) return false; if(op==="$=" && !av.endsWith(v)) return false; }
  return true;
}
function _matches(el,sel){
  for(const part of String(sel||"").split(",")){
    const chain=part.trim().split(/\s+/).filter(Boolean);
    if(!chain.length) continue;
    const last=chain[chain.length-1];
    if(last===">") continue;
    if(_matchSimple(el,_parseSimple(last))){
      if(chain.length===1) return true;
      // ancestor check (loose: ignores > vs descendant distinction)
      let i=chain.length-2, n=el.parentNode, ok=true;
      while(i>=0){ const seg=chain[i]; if(seg===">"){ i--; continue; } const p=_parseSimple(seg); let found=false; while(n){ if(_matchSimple(n,p)){ found=true; n=n.parentNode; break; } n=n.parentNode; } if(!found){ ok=false; break; } i--; }
      if(ok) return true;
    }
  }
  return false;
}
function _walk(root,fn){ for(const c of root.childNodes){ if(c.nodeType===1){ fn(c); _walk(c,fn); } } }
function _qsa(root,sel){ const out=[]; _walk(root,el=>{ if(_matches(el,sel)) out.push(el); }); return out; }

// ---- tiny HTML parser (tags + text + attrs; ignores <script>/<style> bodies as text) ----
export function parseHTML(html){
  const roots=[]; const stack=[];
  const re=/<!--[\s\S]*?-->|<!\[CDATA\[[\s\S]*?\]\]>|<!doctype[^>]*>|<\/([a-zA-Z][\w-]*)\s*>|<([a-zA-Z][\w-]*)((?:\s+[^\s/>"'=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'>]+))?)*)\s*(\/?)>/gi;
  let last=0,m;
  const push=(n)=>{ (stack.length?stack[stack.length-1]:{appendChild:x=>roots.push(x)}).appendChild(n); };
  const text=(s)=>{ if(s) push(new Text_(s.replace(/&amp;/g,"&").replace(/&lt;/g,"<").replace(/&gt;/g,">").replace(/&quot;/g,'"').replace(/&#39;/g,"'").replace(/&nbsp;/g," "))); };
  while((m=re.exec(html))){
    text(html.slice(last,m.index)); last=re.lastIndex;
    if(m[0].startsWith("<!")) continue;
    if(m[1]){ // close
      for(let i=stack.length-1;i>=0;i--){ if(stack[i].localName===m[1].toLowerCase()){ stack.length=i; break; } }
      continue;
    }
    const el=new Element_(m[2]);
    const ar=/([^\s/>"'=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g; let am;
    while((am=ar.exec(m[3]||""))) el.setAttribute(am[1], am[2]??am[3]??am[4]??"");
    push(el);
    if(!m[4] && !VOID.has(el.localName)) stack.push(el);
  }
  text(html.slice(last));
  return roots;
}

class Document_ extends Node_ {
  constructor(){
    super();
    this.nodeType=9; this._ids=new Map();
    this.documentElement=new Element_("html");
    this.head=new Element_("head"); this.body=new Element_("body");
    this.documentElement.appendChild(this.head); this.documentElement.appendChild(this.body);
    this.appendChild(this.documentElement);
    this.activeElement=this.body; this.readyState="complete"; this.visibilityState="visible"; this.hidden=false;
    this.title=""; this.cookie=""; this.adoptedStyleSheets=[]; this.styleSheets=[];
    this.fonts={ ready:Promise.resolve(), add(){}, load(){return Promise.resolve([]);}, check(){return true;}, addEventListener(){}, forEach(){} };
    this.fullscreenElement=null; this.pointerLockElement=null;
  }
  createElement(t){ return new Element_(t); }
  createElementNS(_ns,t){ return new Element_(t); }
  createTextNode(t){ return new Text_(t); }
  createDocumentFragment(){ return new Frag_(); }
  createComment(t){ const n=new Text_(""); n.nodeType=8; n.nodeValue=String(t??""); return n; }
  createRange(){ return { selectNodeContents(){}, setStart(){}, setEnd(){}, collapse(){}, getBoundingClientRect(){return {top:0,left:0,width:0,height:0,bottom:0,right:0};}, cloneRange(){return this;}, deleteContents(){}, insertNode(){}, surroundContents(){}, extractContents(){ return new Frag_(); }, toString(){return "";} }; }
  createTreeWalker(){ return { nextNode:()=>null, currentNode:null }; }
  getElementById(id){ const c=this._ids.get(id); if(c&&_rooted(c,this)) return c; const f=_qsa(this,"#"+id)[0]||null; if(f) this._ids.set(id,f); return f; }
  querySelector(s){ return _qsa(this,s)[0]||null; }
  querySelectorAll(s){ const r=_qsa(this,s); r.forEach=Array.prototype.forEach.bind(r); r.item=(i)=>r[i]??null; return r; }
  getElementsByClassName(c){ return _qsa(this,"."+c); }
  getElementsByTagName(t){ return _qsa(this,t); }
  elementFromPoint(){ return null; }
  execCommand(){ return true; }
  getSelection(){ return { toString:()=>"", removeAllRanges(){}, addRange(){}, rangeCount:0, getRangeAt(){ return _doc.createRange(); }, anchorNode:null, focusNode:null, isCollapsed:true }; }
  exitFullscreen(){ return Promise.resolve(); }
  hasFocus(){ return true; }
}
function _rooted(n,doc){ while(n){ if(n===doc) return true; n=n.parentNode; } return false; }

export function makeDom(indexHtml){
  const doc=new Document_(); _doc=doc;
  if(indexHtml){
    const bodyM=/<body[^>]*>([\s\S]*?)<\/body>/i.exec(indexHtml);
    if(bodyM) parseHTML(bodyM[1]).forEach(n=>doc.body.appendChild(n));
    const headM=/<head[^>]*>([\s\S]*?)<\/head>/i.exec(indexHtml);
    if(headM) parseHTML(headM[1]).forEach(n=>doc.head.appendChild(n));
  }
  return doc;
}
export { Element_, Text_, Frag_, Ev, ClassList, Document_ };
