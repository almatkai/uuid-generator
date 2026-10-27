// Validation only. Transpile and EXECUTE byte-exact own TS. No copied expected algorithm.
const fs=require('node:fs'),path=require('node:path'),Module=require('node:module'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'..'),vendor=path.join(__dirname,'vendor');
const ts=require(path.join(vendor,'typescript/lib/typescript.js'));
let byte=0,now=1700000000000,fixed,events=[],storage={},prefs={},uuid;
function fill(b){for(let i=0;i<b.length;i++){b[i]=fixed??(byte++%256);}return b;}
crypto.randomFillSync=fill;crypto.randomBytes=n=>fill(Buffer.alloc(n));
crypto.randomUUID=()=>uuid.v4({random:fill(new Uint8Array(16))});
Object.defineProperty(globalThis,'crypto',{value:{getRandomValues:fill},configurable:true});
const RealDate=Date;global.Date=class extends RealDate {constructor(...a){super(...(a.length?a:[now]));}static now(){return now;}};
const api={getPreferenceValues:()=>prefs,Clipboard:{copy:async s=>events.push(['copy',s]),paste:async s=>events.push(['paste',s])},showHUD:async s=>events.push(['hud',s]),showToast:async (...a)=>events.push(['toast',...a]),Toast:{Style:{Failure:'failure',Success:'success'}},LocalStorage:{getItem:async k=>storage[k],setItem:async(k,v)=>{storage[k]=v;},removeItem:async k=>{delete storage[k];}},Icon:new Proxy({},{get:(_,k)=>k}),confirmAlert:async options=>{events.push(['confirm',options.title]);options.primaryAction.onAction();}};
let hooks=[],hookIndex=0,pending=[];
const react={useState:initial=>{const i=hookIndex++;if(!(i in hooks))hooks[i]=initial;return [hooks[i],v=>{hooks[i]=typeof v==='function'?v(hooks[i]):v;}];},useEffect:f=>pending.push(f),createElement:(type,props,...children)=>({type,props:props??{},children:children.flat()})};
const component=name=>{const f=()=>{};f.displayName=name;return f;};
for(const [key,subs] of Object.entries({List:['Item'],Action:['CopyToClipboard'],ActionPanel:['Section']})){api[key]=component(key);for(const sub of subs)api[key][sub]=component(key+'.'+sub);}api.Action.Style={Destructive:'destructive'};
const originalResolve=Module._resolveFilename;
Module._resolveFilename=function(request,parent,...rest){if(['uuid','typeid-js','ulidx','react'].includes(request)){
 const name=request==='uuid'&&parent?.filename?.includes('/typeid-js/')?'uuid10':request;
 return originalResolve.call(this,path.join(vendor,name),parent,...rest);
}return originalResolve.call(this,request,parent,...rest);};
const load=Module._load;Module._load=function(request,parent,isMain){if(request==='@raycast/api')return api;if(request==='react')return react;return load.call(this,request,parent,isMain);};
Module._extensions['.ts']=Module._extensions['.tsx']=function(mod,file){const source=fs.readFileSync(file,'utf8');const output=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.React}}).outputText;mod._compile(output,file);};
global.React=react;
function reset(seed=0){for(const k of Object.keys(require.cache))if(k.includes('/oracle/vendor/')&&!k.includes('/typescript/')||k.includes('/fixtures/original/src/'))delete require.cache[k];byte=seed;events=[];storage={};prefs={upperCaseLetters:false,defaultAction:'copy',base32Encoding:false,uuidNamespace:''};uuid=require(path.join(vendor,'uuid'));}
const cases=[];
function add(name,command,args={},preferences={},options={}){cases.push({name,seed:options.seed??0,now:options.now??1700000000000,...(options.fixed!==undefined?{fixed:options.fixed}:{}),steps:[{command,args,prefs:{upperCaseLetters:false,defaultAction:'copy',base32Encoding:false,uuidNamespace:'',...preferences}},...(options.more??[])]});}
const ns='6ba7b810-9dad-11d1-80b4-00c04fd430c8',valid='550e8400-e29b-41d4-a716-446655440000';
for(const command of ['generate','generateV5','generateV7','generateUlid','generateTypeID']){
 const key=command==='generateUlid'?'numberOfULIDsToGenerate':'numberOfUUIDsToGenerate';
 for(const value of [undefined,'','0','-0','1','2','10','  +3junk','1e3','0x10','2.9','\ufeff3','-1','-10000000000000000000','10001','9999999999999999999999999999999999999','Infinity','NaN','no',true,false,0,2,[],[2],{},null])add(`${command}:${JSON.stringify(value)}`,command,{name:'hello 日本 😀',prefix:'user_name',...(value===undefined?{}:{[key]:value})},{uuidNamespace:ns});
 for(const action of ['copy','paste'])for(const upperCaseLetters of [false,true])add(`${command}:${action}:${upperCaseLetters}`,command,{name:'café\u0000',prefix:'user',[key]:'4'},{uuidNamespace:ns,defaultAction:action,upperCaseLetters,base32Encoding:true},{seed:13});
}
for(const name of ['','hello','日本語😀','a\u0000b','e\u0301','é','\n\r\t'])for(const namespace of [ns,'00000000-0000-0000-0000-000000000000','ffffffff-ffff-ffff-ffff-ffffffffffff',valid.toUpperCase(),'bad','550e8400-e29b-01d4-a716-446655440000'])add('v5 name/ns '+JSON.stringify([name,namespace]),'generateV5',{name,numberOfUUIDsToGenerate:'2'},{uuidNamespace:namespace});
for(const prefix of ['','a','a_b','a__b','a'.repeat(63),'a'.repeat(64),'A','_a','a_','1','é','a-b']){add('prefix '+prefix,'generateTypeID',{prefix});add('pack '+prefix,'packTypeID',{suffix:prefix,uuid:valid},{upperCaseLetters:true});}
for(const uuid of [valid,valid.toUpperCase(),'00000000-0000-0000-0000-000000000000','ffffffff-ffff-ffff-ffff-ffffffffffff','550e8400-e29b-01d4-a716-446655440000','550e8400-e29b-91d4-a716-446655440000','550e8400-e29b-41d4-7716-446655440000','short',''])add('pack uuid '+uuid,'packTypeID',{suffix:'doctor',uuid});
for(const s of ['',valid,valid.replaceAll('-',''),valid.toUpperCase(),'!'+valid+'!','zz'+valid+'zz','abc'+valid,'short','é'.repeat(32)])add('format '+s,'formatUuid',{uuid:s},{upperCaseLetters:true});
reset();const typeid=require(path.join(vendor,'typeid-js'));let good=typeid.TypeID.fromUUIDBytes('user',uuid.parse(valid)).toString();
for(const s of ['',good,good.split('_')[1],good.toUpperCase(),'_'+good.split('_')[1],'A_'+good.split('_')[1],'a_','a_7'+'z'.repeat(25),'a_8'+'0'.repeat(25),'a_0'+'i'.repeat(25),'a_0'+'I'.repeat(25),'a_00000000000000000000000000','a_7zzzzzzzzzzzzzzzzzzzzzzzzz','a_00000000000000000000000001'])add('parse '+s,'parseTypeID',{typeID:s});
for(const command of ['generateV7','generateUlid','generateTypeID'])for(const fixedByte of [0,127,255]){
 const key=command==='generateUlid'?'numberOfULIDsToGenerate':'numberOfUUIDsToGenerate';const p={upperCaseLetters:false,defaultAction:'copy',base32Encoding:false};
 add(`${command} repeated/backclock ${fixedByte}`,command,{prefix:'item',[key]:'3'},p,{fixed:fixedByte,more:[{command,args:{prefix:'item',[key]:'2'},prefs:p,now:1699999999999},{command,args:{prefix:'item',[key]:'2'},prefs:p,now:1699999980000},{command,args:{prefix:'item',[key]:'2'},prefs:p,now:1700000000100}]});
}
for(const command of ['generate','generateV5','generateV7','generateUlid','generateTypeID']){
 const key=command==='generateUlid'?'numberOfULIDsToGenerate':'numberOfUUIDsToGenerate';
 for(const value of ['10000','\u00851','\u200b1',1e21,1e-7,'9999x','+0','-0junk']) {
  if(value==='9999x')continue; // count boundary 10000 already exercises the allocation limit.
  add(`boundary ${command}:${JSON.stringify(value)}`,command,{name:'x',prefix:'user',[key]:value},{uuidNamespace:ns});
 }
}
for(const command of ['generateV7','generateUlid','generateTypeID'])for(const now of [0,1,281474976710655])add(`clock ${command}:${now}`,command,{prefix:'user',numberOfUUIDsToGenerate:'2',numberOfULIDsToGenerate:'2'},{},{now,fixed:0});
for(const prefix of ['bad_', '_bad','A'])add('no prefix validation at count zero '+prefix,'generateTypeID',{prefix,numberOfUUIDsToGenerate:'0'});
for(const n of ['-1','10001','bad'])add('v5 validation precedence '+n,'generateV5',{name:'x',numberOfUUIDsToGenerate:n},{uuidNamespace:'bad'});

for(const seq of [0,524287,2147483647]){
 add(`actual TypeID sequence rollover ${seq}`,'generateTypeID',{prefix:'user',numberOfUUIDsToGenerate:'3'});
 cases.at(-1).prelude={typeV7Seq:seq};
}
// Execute asynchronously, retaining module monotonic state within a case.
(async()=>{
 let compared=0;
 for(const c of cases){if(c.name.startsWith('boundary')&&c.name.includes('10000'))console.log('Executing original count=10000:',c.name);fixed=c.fixed;now=c.now;reset(c.seed);if(c.prelude)require(path.join(vendor,'uuid10')).v7({msecs:now,seq:c.prelude.typeV7Seq,random:new Uint8Array(16)});for(const step of c.steps){now=step.now??now;prefs=step.prefs;events=[];storage={};let thrown;try{await require(path.join(root,'fixtures/original/src',step.command)).default({arguments:structuredClone(step.args)});}catch(e){thrown=String(e);}
  const effect=events.find(e=>e[0]==='copy'||e[0]==='paste'),hud=events.find(e=>e[0]==='hud'),toast=events.find(e=>e[0]==='toast');
  if(effect&&hud){step.output={text:effect[1],entries:storage.uuidHistory?JSON.parse(storage.uuidHistory):[],message:hud[1],action:effect[0]};}else{step.originalError={events,thrown:thrown??null};if(toast){step.errorTitle=toast[1].title;}}
  delete step._fn;compared++;
 }}
 fs.writeFileSync(path.join(__dirname,'cases.json'),JSON.stringify(cases,null,2)+'\n');
 // Actual locked uuid11 state helper and byte encoder, including signed sequence wrap.
 const helper=require(path.join(vendor,'uuid/dist/cjs/v7.js'));const low=[];
 for(const seq of [0,2147483647,4294967295])for(const clock of [1699999999999,1700000000000,1700000000001]){
  const before={msecs:1700000000000,seq:seq|0},random=Uint8Array.from({length:16},(_,i)=>i);
  const state=helper.updateV7State({...before},clock,random);
  const result=uuid.v7({msecs:state.msecs,seq:state.seq,random});
  low.push({before,now:clock,random:Array.from(random),state,result});
 }
 fs.writeFileSync(path.join(__dirname,'low-level.json'),JSON.stringify(low,null,2)+'\n');
 // Execute actual original history module and component actions with recording React hooks.
 reset();fixed=undefined;const hist=require(path.join(root,'fixtures/original/src/uuidHistory.ts'));
 await hist.addToHistory('first','uuidV4');await hist.addToHistory('second','uuidV7');await hist.addToHistory('first','uuidV4');const rawInitial=storage.uuidHistory;
 assert.deepEqual((await hist.getHistory()).map(e=>e.uuid),['first','second','first']);
 hooks=[];pending=[];hookIndex=0;const View=require(path.join(root,'fixtures/original/src/viewHistory.tsx')).default;View();for(const f of pending)await f();await new Promise(r=>setImmediate(r));hookIndex=0;let tree=View();assert.deepEqual(tree.children.map(x=>x.props.title),['first','second','first']);
 const panel=tree.children[0].props.actions;await panel.children[1].props.onAction();assert.equal(events.at(-2)[1],'first\r\nsecond\r\nfirst');
 await panel.children[2].children[0].props.onAction();hookIndex=0;tree=View();assert.deepEqual(tree.children.map(x=>x.props.title),['second']);const rawAfterDelete=storage.uuidHistory;
 await tree.children[0].props.actions.children[2].children[1].props.onAction();await new Promise(r=>setImmediate(r));assert.equal(storage.uuidHistory,undefined);
 storage.uuidHistory='{broken';assert.deepEqual(await hist.getHistory(),[]);assert.equal(events.at(-1)[1].title,'Error fetching history');
 storage.uuidHistory=4;await hist.addToHistory('ignored','ulid');assert.equal(storage.uuidHistory,4);await hist.getHistory();assert.equal(events.at(-1)[1].title,'Error fetching history');
 fs.writeFileSync(path.join(__dirname,'history-recording.json'),JSON.stringify({checks:9,rawInitial,rawAfterDelete,events},null,2)+'\n');
 console.log(`Actual original oracle: ${cases.length} cases, ${compared} command steps, 9 history/UI assertions`);
})().catch(e=>{console.error(e);process.exitCode=1;});
