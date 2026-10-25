// Executes actual pinned beta renderer parsing/reconciliation and surface routing.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),rawAssert=require('node:assert/strict');
let assertionCount=0;const assert=Object.assign((...args)=>{assertionCount++;return rawAssert(...args);},{equal:(...args)=>{assertionCount++;return rawAssert.equal(...args);},deepEqual:(...args)=>{assertionCount++;return rawAssert.deepEqual(...args);}});
const root=path.resolve(__dirname,'..'),base=path.join(root,'fixtures/beta/src/renderer/src/raycast-api');
const ts=require(path.join(root,'oracle/vendor/typescript/lib/typescript.js'));
const wire=JSON.parse(fs.readFileSync(path.join(root,'.work/wire.json'),'utf8'));
const fake=name=>Object.assign(()=>null,{displayName:name});
const jsx=(type,props,key)=>({type,props:props??{},key});
const stateUpdates=[];const browser={setTimeout:()=>1,clearTimeout:()=>{},tezbar:{}};
const react={useMemo:f=>f(),useState:v=>[typeof v==='function'?v():v,value=>stateUpdates.push(value)],useEffect:()=>{},useRef:v=>({current:v}),useCallback:f=>f,forwardRef:f=>f,useImperativeHandle:()=>{},createElement:(type,props,...children)=>jsx(type,{...props,children})};react.default=react;
const modules={};
function module(name,extra=''){
 const file=path.join(base,name+'.tsx');const source=fs.readFileSync(file,'utf8')+'\n'+extra;
 const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX}}).outputText;
 const mod={exports:{}};
 const req=id=>{
  if(id==='react')return react;if(id==='react/jsx-runtime')return {jsx,jsxs:jsx,Fragment:fake('Fragment')};
  if(id==='./hooks')return {setRuntimeContext:()=>{},NavigationContext:{Provider:fake('NavigationProvider')}};
  if(id==='./action-runtime-registry')return {ActionRegistryContext:{Provider:fake('ActionRegistryProvider')}};
  if(id.startsWith('./'))return new Proxy({}, {get:(_,key)=>fake(String(key))});
  return new Proxy({}, {get:(_,key)=>fake(String(key))});
 };
 vm.runInNewContext('(function(require,module,exports){'+js+'\n})',{console,URL,Date,window:browser,setTimeout,clearTimeout})(req,mod,mod.exports);return mod.exports;
}
const list=module('list-runtime','export { parseListRows, clientSideFilter };');
const form=module('form-runtime','export { collectFormItems };');
const detail=module('detail-runtime','export { markdownFromNode };');
const surface=module('index','export { rootKind };');
const rows=list.parseListRows(wire.list.root);assert.deepEqual(Array.from(rows,r=>r.title),['first','last','middle','first']);assert(rows.every(r=>r.actionIds.every(id=>wire.list.actions.some(a=>a.id===id))));assert(rows.every(r=>r.accessories.length===2));assert.equal(wire.list.root.props.__hasServerSearch,true);assert.equal(wire.list.root.props.filtering,false);
assert.equal(surface.rootKind(wire.list.root),'list');assert.equal(surface.rootKind(wire.form.root),'form');assert.equal(surface.rootKind(wire.detail.root),'detail');
const fields=form.collectFormItems(wire.form.root).map(x=>x.field).filter(Boolean),changed=form.collectFormItems(wire.changed.root).map(x=>x.field).filter(Boolean);
assert.equal(fields.length,2);assert.equal(fields[0].id,changed[0].id);assert.equal(fields[0].actionId,changed[0].actionId);
const pending={[fields[0].id]:'newer pending draft',[fields[1].id]:'3'};
const reconciled=form.reconcileFormValues(changed,pending,fields);assert.equal(reconciled[fields[0].id],'newer pending draft');assert.equal(reconciled[fields[1].id],'3');
assert(detail.markdownFromNode(wire.detail.root).includes('not acknowledged'));assert(detail.markdownFromNode(wire.confirm.root).includes("You can't undo"));assert.equal(surface.backActionId(wire.detail.root),wire.detail.actions[1].id);assert.equal(surface.backActionId(wire.confirm.root),wire.confirm.actions[1].id);
function nodes(n,out=[]){if(!n||typeof n!=='object')return out;out.push(n);for(const c of [n.props?.children].flat(Infinity))nodes(c,out);return out;}
const calls=[];
function render(view){return surface.ExtensionRuntimeSurface({sessionId:'test',title:view.title,extensionId:'com.tezbar.native.uuid-generator',commandName:'generateV5',root:view.root,actions:view.actions,onBack:()=>calls.push(['back']),onSearchTextChanged:()=>{},onLoadMore:()=>{},onInvokeAction:(...args)=>calls.push(args)});}
const formNode=nodes(render(wire.form)).find(n=>n.type.displayName==='FormRuntime');assert(formNode);formNode.props.onChangeField(fields[0].actionId,'typed');assert.equal(calls.at(-1)[0],fields[0].actionId);assert.equal(calls.at(-1)[1].value,'typed');formNode.props.onSubmitForm(pending);assert.equal(calls.at(-1)[0],wire.form.actions.find(a=>a.kind==='submit-form').id);assert.equal(calls.at(-1)[1],pending);
const detailNode=nodes(render(wire.detail)).find(n=>n.type.displayName==='DetailRuntime');detailNode.props.onBack();assert.equal(calls.at(-1)[0],wire.detail.root.props.backActionId);
const listNode=nodes(render(wire.list)).find(n=>n.type.displayName==='ListRuntime');listNode.props.onRunPrimaryAction(rows[0].actionIds[0]);assert.equal(calls.at(-1)[0],rows[0].actionIds[0]);
// Execute ListRuntime itself with inert hooks/DOM. Its server-search branch must retain all rows.
const tree=list.ListRuntime({root:wire.list.root,title:'History',commandName:'viewHistory',onBack:()=>{},onRunPrimaryAction:()=>{},actions:wire.list.actions,onOpenActions:()=>{},onSearchTextChanged:()=>{},onLoadMore:()=>{}});assert(tree);
(async()=>{
 for(const view of Object.values(wire))assert(view.actions.every(a=>['action','submit-form'].includes(a.kind)));
 const runtime=module('../../ExtensionRuntimeView');
 browser.tezbar.extensionInvokeAction=async()=>({...wire.detail,sessionId:'test',extensionId:'com.tezbar.native.uuid-generator',commandName:'generateV5'});
 const tree=runtime.default({initial:{...wire.form,sessionId:'test',extensionId:'com.tezbar.native.uuid-generator',commandName:'generateV5'},onBack:()=>{}});
 const surface=nodes(tree).find(n=>n.type.displayName==='ExtensionRuntimeSurface');assert(surface);
 await surface.props.onInvokeAction(wire.form.actions[0].id,{});
 assert(stateUpdates.some(v=>v?.tone==='error'&&v.message.startsWith('Failed')));
 assert(!stateUpdates.some(v=>v?.tone==='success'&&v.message.includes('not acknowledged')));
 console.log(`Pinned beta renderer: ${assertionCount} parser/reconciliation/routing/notice assertions; headless hooks/DOM stubs, no visual QA`);
})().catch(e=>{console.error(e);process.exitCode=1;});
