const SKIP = new Set(["then","catch","finally","toJSON","constructor","prototype","length","name","valueOf","inspect","nodeType"]);
export function mk(label = "stub") {
  const fn = function () {};
  fn.__stub = label;
  return new Proxy(fn, {
    get(t, p) {
      if (typeof p === "symbol") {
        if (p === Symbol.toPrimitive) return () => label;
        if (p === Symbol.toStringTag) return label;
        if (p === Symbol.iterator) return function*(){};
        return undefined;
      }
      if (SKIP.has(p)) return Reflect.get(t, p);
      if (!(p in t)) t[p] = mk(label + "." + p);
      return t[p];
    },
    set(t, p, v) { t[p] = v; return true; },
    has() { return true; },
    apply() { return mk(label + "()"); },
    construct() { return mk("new " + label); },
  });
}
// Targeted real implementations for stubbed APIs that MUST return real values.
// A bare proxy here silently corrupts behaviour (e.g. `.colorize(...).then` is not a
// function), which would otherwise be misread as an application defect.
export function mkMonaco(){
  const R = mk("monaco");
  const esc = (s) => String(s).replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
  R.editor.colorize = (code) => Promise.resolve("<span>" + esc(code) + "</span>");
  R.editor.colorizeElement = () => Promise.resolve();
  R.editor.tokenize = (code) => String(code).split("\n").map(() => []);
  R.editor.getModelMarkers = () => [];
  R.editor.getModels = () => [];
  R.editor.getEditors = () => [];
  R.editor.setModelMarkers = () => {};
  R.editor.defineTheme = () => {};
  R.editor.setTheme = () => {};
  R.languages.getLanguages = () => [];
  R.languages.getEncodedLanguageId = () => 1;
  R.Uri.parse = (s) => ({ toString:()=>String(s), path:String(s), fsPath:String(s), scheme:"file" });
  R.Uri.file = (s) => ({ toString:()=>"file://"+s, path:String(s), fsPath:String(s), scheme:"file" });
  return R;
}
const root = mk("monaco");
export default root;
